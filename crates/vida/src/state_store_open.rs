use super::*;

impl StateStore {
    pub async fn open(root: PathBuf) -> Result<Self, StateStoreError> {
        fs::create_dir_all(&root)?;

        for attempt in 0..80 {
            match Self::open_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt < 79 => {
                    let message = error.to_string();
                    if message.contains("LOCK") || message.contains("lock") {
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_once(root).await
    }

    pub async fn open_existing(root: PathBuf) -> Result<Self, StateStoreError> {
        if !root.exists() {
            return Err(StateStoreError::MissingStateDir(root));
        }

        for attempt in 0..80 {
            match Self::open_existing_once(root.clone()).await {
                Ok(store) => return Ok(store),
                Err(StateStoreError::Db(error)) if attempt < 79 => {
                    let message = error.to_string();
                    if message.contains("LOCK") || message.contains("lock") {
                        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                        continue;
                    }
                    return Err(StateStoreError::Db(error));
                }
                Err(error) => return Err(error),
            }
        }

        Self::open_existing_once(root).await
    }

    async fn open_existing_once(root: PathBuf) -> Result<Self, StateStoreError> {
        Self::open_once(root).await
    }

    async fn open_once(root: PathBuf) -> Result<Self, StateStoreError> {
        let db: Surreal<Db> = Surreal::new::<SurrealKv>(root.clone()).await?;
        db.use_ns(STATE_NAMESPACE).use_db(STATE_DATABASE).await?;
        db.query(state_schema_document()).await?;

        let store = Self { db, root };
        store.ensure_minimal_authoritative_state_spine().await?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
