use crate::models::sse::SseEvent;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum AgentCommand {
    ProcessInteraction { user_id: Uuid },
    Shutdown,
}

#[derive(Clone)]
pub struct AgentHandle {
    pub command_tx: mpsc::Sender<AgentCommand>,
    pub event_tx: broadcast::Sender<SseEvent>,
}

pub struct AgentRegistry {
    pub active_agents: scc::HashMap<Uuid, AgentHandle>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            active_agents: scc::HashMap::new(),
        }
    }

    pub async fn get_handle(&self, chat_id: &Uuid) -> Option<AgentHandle> {
        self.active_agents
            .read_async(chat_id, |_, h| h.clone())
            .await
    }

    pub async fn register(&self, chat_id: Uuid, handle: AgentHandle) {
        let _ = self.active_agents.insert_async(chat_id, handle).await;
    }

    pub async fn remove(&self, chat_id: &Uuid) {
        let _ = self.active_agents.remove_async(chat_id).await;
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
