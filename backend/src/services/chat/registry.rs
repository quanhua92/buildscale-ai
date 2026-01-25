use crate::models::sse::SseEvent;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

const EVENT_BUS_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub enum AgentCommand {
    ProcessInteraction { user_id: Uuid },
    Ping,
    Shutdown,
}

#[derive(Clone)]
pub struct AgentHandle {
    pub command_tx: mpsc::Sender<AgentCommand>,
    pub event_tx: broadcast::Sender<SseEvent>,
}

pub struct AgentRegistry {
    pub active_agents: scc::HashMap<Uuid, AgentHandle>,
    pub event_buses: scc::HashMap<Uuid, broadcast::Sender<SseEvent>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            active_agents: scc::HashMap::new(),
            event_buses: scc::HashMap::new(),
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
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
