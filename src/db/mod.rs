use crate::config::Settings;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::{Client, config::Builder};

pub mod chapter;
pub mod job;
pub mod slide;
pub mod subject;
pub mod user;

#[derive(Clone, Debug)]
pub struct Database {
    pub client: Client,
}

impl Database {
    pub async fn new(settings: &Settings) -> Self {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(settings.aws_region.clone()))
            .load()
            .await;

        let builder = Builder::from(&sdk_config);
        let client = Client::from_conf(builder.build());

        Database { client }
    }
}
