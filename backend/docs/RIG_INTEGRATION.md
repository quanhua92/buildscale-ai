# Rig Integration: The AI Execution Runtime

This document specifies the integration between BuildScale's **Workspace OS** and the **Rig.rs** library. To maintain architectural purity, all components interacting with Rig are prefixed with **`Rig`**.

## 1. The Separation of Concerns

BuildScale handles **State and Context** (The OS), while Rig handles **Execution and Orchestration** (The AI).

| Responsibility | Component | Layer |
| :--- | :--- | :--- |
| Persistent Memory | `ContextMap` / PostgreSQL | BuildScale OS |
| File Identity | `files` table / NVMe Mirror | BuildScale OS |
| AI Reasoning | `rig::agent::Agent` | Rig Runtime |
| Tool Execution | `rig::tool::Tool` | Rig Runtime |

## 2. Rig Prefix Mandate

All integration logic must reside in `backend/src/services/chat/` and use the `Rig` prefix:
- `RigLsTool`, `RigReadTool`, etc.: Specific wrappers for BuildScale tools.
- `RigService`: The factory and orchestrator for AI agents.

## 3. The Tool Bridge (`RigTool`)

Rig 0.29 requires tools to implement the `rig::tool::Tool` trait. We define specific wrappers for each core BuildScale tool to maintain type safety and handle async execution through the `DbConn` mutex.

```rust
pub struct RigLsTool {
    pub conn: Arc<Mutex<DbConn>>,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

impl rig::tool::Tool for RigLsTool {
    type Error = Error;
    type Args = LsArgs;
    type Output = serde_json::Value;

    const NAME: &'static str = "ls";

    async fn definition(&self, _prompt: String) -> ToolDefinition { ... }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> { ... }
}
```

## 4. Context-to-Agent Mapping

The `RigService` maps our `ChatSession` and `ContextMap` to a Rig agent:

1. **Model Selection**: Uses `openai::GPT_4O` by default or the model specified in `AgentConfig`.
2. **Preamble**: `ContextKey::SystemPersona` ➔ `builder.preamble()`.
3. **Tools**: Dynamically registered using `builder.tool()`.
4. **Agent Construction**: Returns an `Agent<ResponsesCompletionModel>`.

## 5. SSE Event Mapping

Rig's streaming output is translated into our standardized SSE protocol:
- **Thought**: Internal reasoning/chain-of-thought.
- **Call**: Tool invocation details.
- **Chunk**: Incremental message content.
- **Done**: Final usage stats and metadata.
