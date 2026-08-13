//! Tracks live `SftpClient`s by an opaque id, the same shape as the session
//! adapter's registry but simpler: SFTP has no event stream to pump, just
//! request/response calls, so there's no background task per client — each
//! command borrows the client from the map for the duration of the call.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use portus_sftp::SftpClient;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct SftpState {
    clients: Arc<Mutex<HashMap<String, Arc<SftpClient>>>>,
}

impl SftpState {
    pub fn insert(&self, client: SftpClient) -> String {
        let id = Uuid::new_v4().to_string();
        self.clients.lock().unwrap().insert(id.clone(), Arc::new(client));
        id
    }

    pub fn get(&self, id: &str) -> Result<Arc<SftpClient>, String> {
        self.clients.lock().unwrap().get(id).cloned().ok_or_else(|| "unknown sftp session".to_string())
    }

    pub fn remove(&self, id: &str) {
        self.clients.lock().unwrap().remove(id);
    }
}
