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

The `RigService` maps our `ChatSession` and `BuiltContext` to a Rig agent:

1. **Model Selection**: Uses `openai::GPT_4O` by default or the model specified in `AgentConfig`.
2. **Preamble**: System instructions resolved from the `src/agents` registry based on the session role ➔ `builder.preamble()`.
3. **Tools**: Dynamically registered using `builder.tool()`.
4. **Agent Construction**: Returns an `Agent<ResponsesCompletionModel>`.

## 5. Chat History Management

### 5.1 How Rig Handles Chat History

Rig's `stream_chat` takes a `chat_history: Vec<Message>` parameter and **automatically maintains** tool calls and results during streaming:

```rust
// During streaming, Rig automatically pushes to history:
chat_history.write().await.push(Message::Assistant {
    id: None,
    content: OneOrMany::many(tool_calls),
});

chat_history.write().await.push(Message::User {
    content: OneOrMany::one(UserContent::tool_result_with_call_id(...)),
});
```

**Critical**: This history is **local to the stream** and is lost when the stream completes!

### 5.2 Multi-Turn Conversation Pattern

To maintain context across multiple user interactions:

1. **Interaction 1**: `stream_chat(prompt1, history1)`
   - Rig adds tool calls/results to its local `history1`
   - Stream completes, local history is dropped
   - Tool calls/results are persisted to database as `role: Tool` messages

2. **Interaction 2**: `stream_chat(prompt2, history2)`
   - BuildScale must reconstruct `history2` from database
   - **MUST include previous tool calls/results** from database
   - Current bug: `convert_history` filters out `role: Tool` messages
   - Fix: Reconstruct `Message` with `ToolCall` and `ToolResult` from persisted data

### 5.3 Implementation Requirement

**`convert_history` function** (rig_engine.rs) must reconstruct Rig messages from persisted Tool messages:

```rust
// Reconstruct tool calls from persisted data
Message::assistant(vec![AssistantContent::ToolCall(ToolCall {
    id: tool_id,
    call_id: None,
    function: ToolCallFunction {
        name: tool_name,
        arguments: tool_args,
    },
    signature: None,
})])

// Reconstruct tool results from persisted data
Message::user(vec![UserContent::ToolResult(ToolResult {
    id: tool_id,
    call_id: None,
    content: vec![ToolResultContent::Text(Text {
        text: summarized_output + "\n[Note: Summarized result]"
    })].into(),
})])
```

**Important**: Tool results are stored as **summaries** in the database (e.g., first 5 lines of a 1000-line file) to avoid storage bloat. The summaries are transparently marked so the model knows they're truncated.

## 6. SSE Event Mapping

Rig's streaming output is translated into our standardized SSE protocol:
- **Thought**: Internal reasoning/chain-of-thought.
- **Call**: Tool invocation details.
- **Chunk**: Incremental message content.
- **Done**: Final usage stats and metadata.
