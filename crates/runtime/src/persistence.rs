use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::session::Session;

pub trait SessionStore: Send + Sync + 'static {
    fn save(&mut self, session: &Session) -> Result<(), PersistenceError>;
    fn load(&self, session_id: &str) -> Result<Session, PersistenceError>;
    fn delete(&mut self, session_id: &str) -> Result<(), PersistenceError>;
    fn list_sessions(&self) -> Result<Vec<SessionInfo>, PersistenceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFileSessionStore {
    base_path: PathBuf,
    index: HashMap<String, SessionInfo>,
}

impl JsonFileSessionStore {
    pub fn new<P: AsRef<std::path::Path>>(base_path: P) -> Result<Self, PersistenceError> {
        let path = base_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;

        let index_path = path.join("sessions_index.json");
        let index: HashMap<String, SessionInfo> = if index_path.exists() {
            let data = std::fs::read(index_path)?;
            serde_json::from_slice(&data)?
        } else {
            HashMap::new()
        };

        Ok(Self { base_path: path, index })
    }

    fn save_index(&self) -> Result<(), PersistenceError> {
        let index_path = self.base_path.join("sessions_index.json");
        let data = serde_json::to_vec_pretty(&self.index)?;
        std::fs::write(index_path, data)?;
        Ok(())
    }

    fn session_path(&self, session_id: &str) -> std::path::PathBuf {
        self.base_path.join(format!("{}.json", session_id))
    }
}

impl SessionStore for JsonFileSessionStore {
    fn save(&mut self, session: &Session) -> Result<(), PersistenceError> {
        let session_path = self.session_path(&session.id);
        let data = serde_json::to_vec_pretty(session)?;
        std::fs::write(session_path, data)?;

        let info = SessionInfo::new(session);
        self.index.insert(session.id.clone(), info);
        self.save_index()?;
        Ok(())
    }

    fn load(&self, session_id: &str) -> Result<Session, PersistenceError> {
        let session_path = self.session_path(session_id);
        let data = std::fs::read(session_path)?;
        let session: Session = serde_json::from_slice(&data)?;
        Ok(session)
    }

    fn delete(&mut self, session_id: &str) -> Result<(), PersistenceError> {
        let session_path = self.session_path(session_id);
        if session_path.exists() {
            std::fs::remove_file(session_path)?;
        }
        self.index.remove(session_id);
        self.save_index()?;
        Ok(())
    }

    fn list_sessions(&self) -> Result<Vec<SessionInfo>, PersistenceError> {
        let mut infos: Vec<SessionInfo> = self.index.values().cloned().collect();
        infos.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(infos)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub model: Option<String>,
}

impl SessionInfo {
    pub fn new(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
            status: format!("{:?}", session.status),
            model: session.model.clone(),
        }
    }
}
