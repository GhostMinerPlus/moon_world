use std::sync::Arc;

use edge_lib::engine::EdgeEngine;
use sqlite_dm::SqliteDataManager;

use super::{BodyBuilder, Joint};

pub struct SceneBuilder {
    body_v: Vec<BodyBuilder>,
    joint_v: Vec<Joint>,
    event_handler: Vec<String>,
    step_handler: Vec<String>,
    collision_handler: Vec<String>,
}

impl SceneBuilder {
    pub async fn from_data(file: &str) -> Self {
        let dm = Arc::new(SqliteDataManager::from_file("test.db", None).await);
        let mut engine = EdgeEngine::new(dm, "root").await;

        
        todo!()
    }
}
