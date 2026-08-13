//! Tracks live `RdpClient`s by an opaque id. Unlike the session adapter,
//! there's no command channel here — the only thing the app ever needs to
//! tell a running RDP connection (for now, view-only) is "stop", which
//! `RdpClient::shutdown` handles directly.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use portus_rdp::RdpClient;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct RdpState {
    clients: Arc<Mutex<HashMap<String, Arc<Mutex<RdpClient>>>>>,
}

impl RdpState {
    pub fn reserve_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn insert(&self, id: String, client: RdpClient) {
        self.clients.lock().unwrap().insert(id, Arc::new(Mutex::new(client)));
    }

    pub fn remove(&self, id: &str) {
        if let Some(client) = self.clients.lock().unwrap().remove(id) {
            client.lock().unwrap().shutdown();
        }
    }
}
