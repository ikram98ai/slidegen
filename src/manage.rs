//! manage — CLI tool for managing Slidegen infrastructure
//!
//! Usage:
//!   cargo run --bin manage -- db create-tables
//!   cargo run --bin manage -- db delete-tables
//!   cargo run --bin manage -- s3 create-bucket
//!   cargo run --bin manage -- s3 delete-bucket

use anyhow::{Context, Result};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::{
    client::Client as DynamoClient,
    types::{
        AttributeDefinition, BillingMode, GlobalSecondaryIndex, KeySchemaElement, KeyType,
        Projection, ProjectionType, ScalarAttributeType,
    },
};
use aws_sdk_s3::client::Client as S3Client;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use serde_json::json;
use std::env;

// ──────────────────────────── CLI definitions ─────────────────────────────

#[derive(Parser)]
#[command(
    name = "manage",
    about = "Slidegen infrastructure management CLI",
    version
)]
struct Cli {
    #[command(subcommand)]
    group: Group,
}

#[derive(Subcommand)]
enum Group {
    /// DynamoDB table management
    Db {
        #[command(subcommand)]
        cmd: DbCommand,
    },
    /// S3 bucket management
    S3 {
        #[command(subcommand)]
        cmd: S3Command,
    },
}

#[derive(Subcommand)]
enum DbCommand {
    /// Create all DynamoDB tables required by Slidegen
    CreateTables,
    /// Delete all DynamoDB tables used by Slidegen
    DeleteTables,
}

#[derive(Subcommand)]
enum S3Command {
    /// Create and configure the public S3 bucket
    CreateBucket,
    /// Delete the S3 bucket and all its contents
    DeleteBucket,
}

// ──────────────────────────── DynamoDB helpers ─────────────────────────────

/// Table definitions: (table_name, hash_key, range_key, gsi_definitions)
/// GSI format: (gsi_name, hash_attr, hash_type, range_attr?, range_type?)
struct TableDef {
    name: &'static str,
    hash_key: (&'static str, ScalarAttributeType),
    range_key: Option<(&'static str, ScalarAttributeType)>,
    gsi: Vec<GsiDef>,
}

struct GsiDef {
    name: &'static str,
    hash_key: (&'static str, ScalarAttributeType),
    range_key: Option<(&'static str, ScalarAttributeType)>,
}

fn table_definitions() -> Vec<TableDef> {
    vec![
        TableDef {
            name: "slidegen_users",
            hash_key: ("id", ScalarAttributeType::S),
            range_key: None,
            gsi: vec![GsiDef {
                name: "email-index",
                hash_key: ("email", ScalarAttributeType::S),
                range_key: None,
            }],
        },
        TableDef {
            name: "slidegen_subjects",
            hash_key: ("id", ScalarAttributeType::S),
            range_key: None,
            gsi: vec![GsiDef {
                name: "user-index",
                hash_key: ("user_id", ScalarAttributeType::S),
                range_key: None,
            }],
        },
        TableDef {
            name: "slidegen_chapters",
            hash_key: ("subject_id", ScalarAttributeType::S),
            range_key: Some(("id", ScalarAttributeType::S)),
            gsi: vec![],
        },
        TableDef {
            name: "slidegen_slides",
            hash_key: ("chapter_id", ScalarAttributeType::S),
            range_key: Some(("id", ScalarAttributeType::S)),
            gsi: vec![],
        },
    ]
}

async fn create_tables(client: &DynamoClient) -> Result<()> {
    for table in table_definitions() {
        // Check if table exists
        let exists = client
            .describe_table()
            .table_name(table.name)
            .send()
            .await
            .is_ok();

        if exists {
            println!("Table '{}' already exists — skipping.", table.name);
            continue;
        }

        println!("Creating table '{}'...", table.name);

        // Build attribute definitions
        let mut attr_defs = vec![
            AttributeDefinition::builder()
                .attribute_name(table.hash_key.0)
                .attribute_type(table.hash_key.1.clone())
                .build()?,
        ];

        // Range key attribute
        if let Some((rk_name, rk_type)) = &table.range_key {
            attr_defs.push(
                AttributeDefinition::builder()
                    .attribute_name(*rk_name)
                    .attribute_type(rk_type.clone())
                    .build()?,
            );
        }

        // Key schema
        let mut key_schema = vec![
            KeySchemaElement::builder()
                .attribute_name(table.hash_key.0)
                .key_type(KeyType::Hash)
                .build()?,
        ];

        if let Some((rk_name, _)) = &table.range_key {
            key_schema.push(
                KeySchemaElement::builder()
                    .attribute_name(*rk_name)
                    .key_type(KeyType::Range)
                    .build()?,
            );
        }

        // Build GSIs
        let mut gsis: Vec<GlobalSecondaryIndex> = vec![];
        for gsi in &table.gsi {
            // Add GSI attribute definitions
            attr_defs.push(
                AttributeDefinition::builder()
                    .attribute_name(gsi.hash_key.0)
                    .attribute_type(gsi.hash_key.1.clone())
                    .build()?,
            );
            if let Some((rk_name, rk_type)) = &gsi.range_key {
                attr_defs.push(
                    AttributeDefinition::builder()
                        .attribute_name(*rk_name)
                        .attribute_type(rk_type.clone())
                        .build()?,
                );
            }

            let mut gsi_key_schema = vec![
                KeySchemaElement::builder()
                    .attribute_name(gsi.hash_key.0)
                    .key_type(KeyType::Hash)
                    .build()?,
            ];

            if let Some((rk_name, _)) = &gsi.range_key {
                gsi_key_schema.push(
                    KeySchemaElement::builder()
                        .attribute_name(*rk_name)
                        .key_type(KeyType::Range)
                        .build()?,
                );
            }

            gsis.push(
                GlobalSecondaryIndex::builder()
                    .index_name(gsi.name)
                    .set_key_schema(Some(gsi_key_schema))
                    .projection(
                        Projection::builder()
                            .projection_type(ProjectionType::All)
                            .build(),
                    )
                    .build()?,
            );
        }

        let mut req = client
            .create_table()
            .table_name(table.name)
            .set_attribute_definitions(Some(attr_defs))
            .set_key_schema(Some(key_schema))
            .billing_mode(BillingMode::PayPerRequest);

        for gsi in gsis {
            req = req.global_secondary_indexes(gsi);
        }

        req.send()
            .await
            .with_context(|| format!("Failed to create table '{}'", table.name))?;

        println!("✓ Table '{}' created successfully.", table.name);
    }
    Ok(())
}

async fn delete_tables(client: &DynamoClient) -> Result<()> {
    for table in table_definitions() {
        let exists = client
            .describe_table()
            .table_name(table.name)
            .send()
            .await
            .is_ok();

        if !exists {
            println!("Table '{}' does not exist — skipping.", table.name);
            continue;
        }

        println!("Deleting table '{}'...", table.name);
        client
            .delete_table()
            .table_name(table.name)
            .send()
            .await
            .with_context(|| format!("Failed to delete table '{}'", table.name))?;

        println!("✓ Table '{}' deleted.", table.name);
    }
    Ok(())
}

// ──────────────────────────── S3 helpers ──────────────────────────────────

async fn create_bucket(s3: &S3Client, bucket: &str, region: &str) -> Result<()> {
    use aws_sdk_s3::types::{
        BucketLocationConstraint, CreateBucketConfiguration, PublicAccessBlockConfiguration,
    };

    println!("Creating bucket '{}'...", bucket);

    // Create bucket (us-east-1 must NOT send a LocationConstraint, others must)
    let mut req = s3.create_bucket().bucket(bucket);
    if region != "us-east-1" {
        req = req.create_bucket_configuration(
            CreateBucketConfiguration::builder()
                .location_constraint(BucketLocationConstraint::from(region))
                .build(),
        );
    }
    req.send()
        .await
        .with_context(|| format!("Failed to create bucket '{}'", bucket))?;
    println!("✓ Bucket '{}' created.", bucket);

    // Disable block public access
    println!("Disabling public access block...");
    s3.put_public_access_block()
        .bucket(bucket)
        .public_access_block_configuration(
            PublicAccessBlockConfiguration::builder()
                .block_public_acls(false)
                .ignore_public_acls(false)
                .block_public_policy(false)
                .restrict_public_buckets(false)
                .build(),
        )
        .send()
        .await
        .with_context(|| "Failed to disable public access block")?;
    println!("✓ Public access block disabled.");

    // Apply public-read bucket policy
    println!("Applying public read policy...");
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Sid": "PublicReadGetObject",
            "Effect": "Allow",
            "Principal": "*",
            "Action": "s3:GetObject",
            "Resource": format!("arn:aws:s3:::{}/*", bucket)
        }]
    });

    s3.put_bucket_policy()
        .bucket(bucket)
        .policy(serde_json::to_string(&policy)?)
        .send()
        .await
        .with_context(|| "Failed to apply bucket policy")?;
    println!("✓ Public read policy applied to '{}'.", bucket);

    Ok(())
}

async fn delete_bucket(s3: &S3Client, bucket: &str) -> Result<()> {
    use aws_sdk_s3::types::ObjectIdentifier;

    // List and delete all object versions + delete markers
    println!("Listing all objects and versions in '{}'...", bucket);

    let mut objects_to_delete: Vec<ObjectIdentifier> = vec![];
    let mut key_marker: Option<String> = None;
    let mut version_id_marker: Option<String> = None;

    loop {
        let mut req = s3.list_object_versions().bucket(bucket);
        if let Some(ref km) = key_marker {
            req = req.key_marker(km.clone());
        }
        if let Some(ref vim) = version_id_marker {
            req = req.version_id_marker(vim.clone());
        }

        let page = req
            .send()
            .await
            .with_context(|| format!("Failed to list object versions in '{}'", bucket))?;

        for v in page.versions() {
            let key = v.key().unwrap_or_default().to_string();
            let vid = v.version_id().unwrap_or_default().to_string();
            objects_to_delete.push(
                ObjectIdentifier::builder()
                    .key(key)
                    .version_id(vid)
                    .build()?,
            );
        }

        for dm in page.delete_markers() {
            let key = dm.key().unwrap_or_default().to_string();
            let vid = dm.version_id().unwrap_or_default().to_string();
            objects_to_delete.push(
                ObjectIdentifier::builder()
                    .key(key)
                    .version_id(vid)
                    .build()?,
            );
        }

        if page.is_truncated().unwrap_or(false) {
            key_marker = page.next_key_marker().map(|s| s.to_string());
            version_id_marker = page.next_version_id_marker().map(|s| s.to_string());
        } else {
            break;
        }
    }

    if objects_to_delete.is_empty() {
        println!("Bucket is already empty.");
    } else {
        println!("Deleting {} objects...", objects_to_delete.len());
        // S3 delete_objects handles up to 1000 per call
        for chunk in objects_to_delete.chunks(1000) {
            use aws_sdk_s3::types::Delete;
            s3.delete_objects()
                .bucket(bucket)
                .delete(
                    Delete::builder()
                        .set_objects(Some(chunk.to_vec()))
                        .build()?,
                )
                .send()
                .await
                .with_context(|| "Failed to delete objects")?;
        }
        println!("✓ All objects deleted.");
    }

    println!("Deleting bucket '{}'...", bucket);
    s3.delete_bucket()
        .bucket(bucket)
        .send()
        .await
        .with_context(|| format!("Failed to delete bucket '{}'", bucket))?;
    println!("✓ Bucket '{}' deleted successfully.", bucket);

    Ok(())
}

// ──────────────────────────── Entry point ─────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let aws_region = env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let s3_bucket = env::var("S3_BUCKET_NAME").context("S3_BUCKET_NAME must be set")?;

    // Shared AWS config
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(aws_region.clone()))
        .load()
        .await;

    let dynamo = DynamoClient::new(&sdk_config);
    let s3 = S3Client::new(&sdk_config);

    let cli = Cli::parse();

    match cli.group {
        Group::Db { cmd } => match cmd {
            DbCommand::CreateTables => create_tables(&dynamo).await?,
            DbCommand::DeleteTables => delete_tables(&dynamo).await?,
        },
        Group::S3 { cmd } => match cmd {
            S3Command::CreateBucket => create_bucket(&s3, &s3_bucket, &aws_region).await?,
            S3Command::DeleteBucket => delete_bucket(&s3, &s3_bucket).await?,
        },
    }

    Ok(())
}
