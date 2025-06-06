use std::sync::Arc;

use nostrstore::{Database, QueryOptions, operation::counter::CounterEvent};

use crate::{AppError, Keypair};

#[derive(uniffi::Object)]
pub struct PortalDB {
    database: Arc<Database>,
}

#[uniffi::export]
impl PortalDB {
    #[uniffi::constructor]
    pub async fn new(keypair: Arc<Keypair>, relays: Vec<String>) -> Result<Arc<Self>, AppError> {
        let keypair = &keypair.inner;

        let database = Database::builder(keypair.get_keys().clone())
            .with_relays(relays)
            .build()
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create database: {}", e)))?;

        let database = PortalDB {
            database: Arc::new(database),
        };
        Ok(Arc::new(database))
    }

    pub async fn read(&self, key: String) -> Result<String, AppError> {
        let value = self
            .database
            .read(key)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get value: {}", e)))?;
        Ok(value)
    }

    pub async fn store(&self, key: String, value: &str) -> Result<(), AppError> {
        self.database
            .store(key, value)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to set value: {}", e)))?;
        Ok(())
    }

    pub async fn remove(&self, key: String) -> Result<(), AppError> {
        self.database
            .remove(key)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to remove value: {}", e)))?;
        Ok(())
    }

    pub async fn read_history(&self, key: String) -> Result<Vec<String>, AppError> {
        let history = self
            .database
            .read_history(key, QueryOptions::default())
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get history: {}", e)))?
            .iter()
            .map(|record| record.content.clone())
            .collect::<Vec<String>>();

        Ok(history)
    }

    pub async fn increment_counter(&self, key: String) -> Result<(), AppError> {
        self.database
            .store_event(key, CounterEvent::Increment)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to increment counter: {}", e)))?;
        Ok(())
    }
    pub async fn decrement_counter(&self, key: String) -> Result<(), AppError> {
        self.database
            .store_event(key, CounterEvent::Decrement)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to decrement counter: {}", e)))?;
        Ok(())
    }

    pub async fn read_counter(&self, key: String) -> Result<i64, AppError> {
        let counter = self
            .database
            .read_event::<CounterEvent>(key)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get counter: {}", e)))?;
        Ok(counter)
    }
}
