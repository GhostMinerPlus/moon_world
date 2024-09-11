use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite};
use std::{future, io, pin::Pin, sync::Arc};

use edge_lib::{
    data::{AsDataManager, Auth},
    util::Path,
};

mod dao;

const INIT_SQL: &str = "CREATE TABLE IF NOT EXISTS edge_t (
    id integer PRIMARY KEY,
    source varchar(500),
    paper varchar(100),
    code varchar(100),
    target varchar(500)
);
CREATE INDEX IF NOT EXISTS edge_t_source_paper_code ON edge_t (source, paper, code);
CREATE INDEX IF NOT EXISTS edge_t_target_paper_code ON edge_t (target, paper, code);";

#[derive(Clone)]
pub struct SqliteDataManager {
    pool: Pool<Sqlite>,
    auth: Auth,
}

impl SqliteDataManager {
    pub async fn from_file(file: &str, auth: Auth) -> Self {
        let pool = sqlx::SqlitePool::connect_with(SqliteConnectOptions::new().filename(file))
            .await
            .unwrap();
        Self { pool, auth }
    }

    pub async fn create(file: &str, auth: Auth) -> io::Result<Self> {
        std::fs::File::create_new(file)?;
        let this = Self::from_file(file, auth).await;
        sqlx::query(INIT_SQL)
            .execute(&this.pool)
            .await
            .map_err(|e| io::Error::other(e))?;
        Ok(this)
    }
}

impl AsDataManager for SqliteDataManager {
    fn get_auth(&self) -> &Auth {
        &self.auth
    }

    fn divide(&self, auth: Auth) -> Arc<dyn AsDataManager> {
        Arc::new(Self {
            auth,
            pool: self.pool.clone(),
        })
    }

    fn append(
        &self,
        path: &Path,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>> {
        if path.step_v.is_empty() {
            return Box::pin(future::ready(Ok(())));
        }
        let this = self.clone();
        let mut path = path.clone();
        Box::pin(async move {
            let step = path.step_v.pop().unwrap();
            if let Some(auth) = &this.auth {
                if !auth.writer.contains(&step.paper) {
                    return Err(io::Error::other("permision denied"));
                }
            }
            let root_v = this.get(&path).await?;
            for source in &root_v {
                dao::insert_edge(this.pool.clone(), source, &step.paper, &step.code, &item_v)
                    .await?;
            }
            Ok(())
        })
    }

    fn set(
        &self,
        path: &Path,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>> {
        if path.step_v.is_empty() {
            return Box::pin(future::ready(Ok(())));
        }
        let this = self.clone();
        let mut path = path.clone();
        Box::pin(async move {
            let step = path.step_v.pop().unwrap();
            if let Some(auth) = &this.auth {
                if !auth.writer.contains(&step.paper) {
                    return Err(io::Error::other("permision denied"));
                }
            }
            let root_v = this.get(&path).await?;
            for source in &root_v {
                dao::delete_edge_with_source_code(
                    this.pool.clone(),
                    &step.paper,
                    source,
                    &step.code,
                )
                .await?;
            }
            for source in &root_v {
                dao::insert_edge(this.pool.clone(), source, &step.paper, &step.code, &item_v)
                    .await?;
            }
            Ok(())
        })
    }

    fn get(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = io::Result<Vec<String>>> + Send>> {
        if path.step_v.is_empty() {
            if let Some(root) = &path.root_op {
                return Box::pin(future::ready(Ok(vec![root.clone()])));
            } else {
                return Box::pin(future::ready(Ok(vec![])));
            }
        }
        let this = self.clone();
        let path = path.clone();
        Box::pin(async move {
            if let Some(auth) = &this.auth {
                for step in &path.step_v {
                    if !auth.writer.contains(&step.paper) && !auth.reader.contains(&step.paper) {
                        return Err(io::Error::other("permision denied"));
                    }
                }
            }
            dao::get(this.pool.clone(), &path).await
        })
    }

    fn clear(&self) -> Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            match &this.auth {
                Some(auth) => {
                    for paper in &auth.writer {
                        let _ = dao::clear_paper(this.pool.clone(), paper).await;
                    }
                    Ok(())
                }
                None => dao::clear(this.pool).await,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use edge_lib::engine::{EdgeEngine, ScriptTree1};

    use super::*;

    #[test]
    fn test_root_type() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let dm = Arc::new(SqliteDataManager::from_file("test.db", None).await);
            let mut engine = EdgeEngine::new(dm, "root").await;
            engine
                .execute2(&ScriptTree1 {
                    script: vec!["root->type = user _".to_string()],
                    name: "rs".to_string(),
                    next_v: vec![],
                })
                .await
                .unwrap();
            engine.reset().await.unwrap();

            let rs = engine
                .get_gloabl()
                .get(&Path::from_str("root->type"))
                .await
                .unwrap();
            assert_eq!(rs[0], "user")
        })
    }
}
