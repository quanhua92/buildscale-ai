use crate::error::Result;
use crate::models::sse::SseEvent;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const EVENT_BUS_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub enum AgentCommand {
    ProcessInteraction { user_id: Uuid },
    Ping,
    Shutdown,
    Cancel {
        reason: String,
        responder: Arc<Mutex<Option<oneshot::Sender<Result<bool>>>>>,
    },
}

#[derive(Clone)]
pub struct AgentHandle {
    pub command_tx: mpsc::Sender<AgentCommand>,
    pub event_tx: broadcast::Sender<SseEvent>,
}

pub struct AgentRegistry {
    pub active_agents: scc::HashMap<Uuid, AgentHandle>,
    pub event_buses: scc::HashMap<Uuid, broadcast::Sender<SseEvent>>,
    /// Track active cancellation tokens for streams
    /// This allows STOP to cancel streams even after actor exits
    pub active_cancellations: Arc<Mutex<HashMap<Uuid, CancellationToken>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            active_agents: scc::HashMap::new(),
            event_buses: scc::HashMap::new(),
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Gets or creates a persistent broadcast bus for a chat.
    /// This bus survives actor restarts to keep SSE connections stable.
    pub async fn get_or_create_bus(&self, chat_id: Uuid) -> broadcast::Sender<SseEvent> {
        if let Some(bus) = self.event_buses.read_async(&chat_id, |_, b| b.clone()).await {
            bus
        } else {
            tracing::info!("Creating new persistent event bus for chat {}", chat_id);
            let (tx, _) = broadcast::channel(EVENT_BUS_CAPACITY);
            let _ = self.event_buses.insert_async(chat_id, tx.clone()).await;
            tx
        }
    }

    pub async fn get_handle(&self, chat_id: &Uuid) -> Option<AgentHandle> {
        let handle = self.active_agents
            .read_async(chat_id, |_, h| h.clone())
            .await?;

        // Check if the actor is still alive (receiver hasn't dropped)
        if handle.command_tx.is_closed() {
            // Remove the dead handle and return None to trigger re-spawn
            let _ = self.active_agents.remove_async(chat_id).await;
            None
        } else {
            Some(handle)
        }
    }

    pub async fn register(&self, chat_id: Uuid, handle: AgentHandle) {
        tracing::info!("Registering active actor for chat {}", chat_id);
        let _ = self.active_agents.insert_async(chat_id, handle).await;
    }

    pub async fn remove(&self, chat_id: &Uuid) {
        tracing::info!("Removing actor for chat {}", chat_id);
        let _ = self.active_agents.remove_async(chat_id).await;
    }

    /// Register a cancellation token for an active stream
    pub async fn register_cancellation(&self, chat_id: Uuid, token: CancellationToken) {
        tracing::info!("Registering cancellation token for stream {}", chat_id);
        let mut cancellations = self.active_cancellations.lock().await;
        cancellations.insert(chat_id, token);
    }

    /// Remove a cancellation token when stream completes
    pub async fn remove_cancellation(&self, chat_id: &Uuid) {
        tracing::debug!("Removing cancellation token for stream {}", chat_id);
        let mut cancellations = self.active_cancellations.lock().await;
        cancellations.remove(chat_id);
    }

    /// Cancel an active stream by chat ID
    /// Returns true if a token was found and cancelled, false otherwise
    pub async fn cancel_stream(&self, chat_id: &Uuid) -> bool {
        let mut cancellations = self.active_cancellations.lock().await;
        if let Some(token) = cancellations.remove(chat_id) {
            tracing::info!("Cancelling active stream for chat {}", chat_id);
            token.cancel();
            true
        } else {
            tracing::debug!("No active cancellation token found for chat {}", chat_id);
            false
        }
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
