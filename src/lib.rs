pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod mcp;
pub mod models;
pub mod services;

use std::sync::Arc;

use crate::config::Settings;
use crate::db::Database;
use crate::services::retrieval::RetrievalService;
use crate::services::{AIService, BackgroundTasksService, StorageService};

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub db: Arc<Database>,
    pub storage: Arc<StorageService>,
    pub ai: Arc<AIService>,
    pub bg_tasks: Arc<BackgroundTasksService>,
    pub retrieval: Arc<RetrievalService>,
}
