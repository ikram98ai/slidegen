//! slidegen-mcp — stdio MCP server for search_book / get_chapter /
//! get_scene / get_citation.
//!
//! Env: SLIDEGEN_BASE_URL (default http://127.0.0.1:8000), SLIDEGEN_API_KEY.

use anyhow::{Context, Result};
use slidegen::mcp::{SlidegenHttp, handle_line};
use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "slidegen=info".into()),
        )
        .init();

    let base =
        std::env::var("SLIDEGEN_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());
    let api_key = std::env::var("SLIDEGEN_API_KEY").context("SLIDEGEN_API_KEY must be set")?;
    let client = SlidegenHttp::new(base, api_key);

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = std::io::stdout();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(out) = handle_line(&client, &line).await {
            writeln!(stdout, "{out}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
