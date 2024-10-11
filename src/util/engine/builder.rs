use std::sync::Arc;

use sqlite_dm::SqliteDataManager;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

use super::{BodyBuilder, Joint};

pub struct SceneBuilder {
    body: Vec<BodyBuilder>,
    joint: Vec<Joint>,
    event_handler: Vec<String>,
    step_handler: Vec<String>,
    collision_handler: Vec<String>,
}

impl SceneBuilder {
    pub async fn from_file(file: &str) -> Self {
        let pool = SqlitePool::connect_with(SqliteConnectOptions::new().filename(file))
            .await
            .unwrap();
        let dm = Arc::new(SqliteDataManager::new(pool, None));

        todo!()
    }
}
