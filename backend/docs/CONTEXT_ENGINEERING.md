# Context Engineering: The Cortex Guide

BuildScale.ai treats LLM context as a dynamically engineered resource, managed through a structured **BuiltContext** architecture. This replaces traditional string concatenation with a properly integrated, multi-modal context system that leverages the Rig.rs AI framework's native capabilities.

## 1. The Core Philosophy

The "Workspace is the OS" vision requires that the Agentic Engine can precisely control what an LLM "sees" at any given moment. By using **structured context** instead of string concatenation, we achieve:

- **Framework Integration**: Persona, history, and attachments are passed to Rig in their native formats.
- **Type Safety**: No more string parsing or XML marker escaping issues.
- **Positional Integrity**: The framework ensures proper ordering (System -> History -> Request with Attachments).
- **Graceful Degradation**: Foundation for intelligent pruning when hitting token limits (TODO).

## 2. Anatomy of BuiltContext

### The Struct (Current Implementation)

```rust
pub struct BuiltContext {
    /// System persona/instructions for the AI
    pub persona: String,
    /// Conversation history (excluding current message)
    pub history: Vec<ChatMessage>,
    /// File attachments with their content
    pub attachments: Vec<FileAttachment>,
}

pub struct FileAttachment {
    pub file_id: Uuid,
    pub path: String,
    pub content: String,
}
```

### How Each Component Flows to the AI

| Component | Source | Destination in Rig Pipeline |
| :--- | :--- | :--- |
| **persona** | Hardcoded string or agent config | `AgentBuilder::with_system_prompt()` |
| **history** | Previous messages from DB | `agent.stream_chat(prompt, history)` |
| **attachments** | File IDs in message metadata | Appended to user prompt string |
| **current_message** | Last message in DB | Passed as the `prompt` parameter |

### Why This Structure Works

1. **Persona**: Set as the system prompt via `persona_override` in agent config
2. **History**: Converted to Rig's `ChatMessage` format and passed to the multi-turn agent
3. **Attachments**: Formatted as text blocks and appended to the current user prompt
4. **Current Message**: The prompt that triggers the AI response

## 3. The Lifecycle: From IDs to AI

### Phase 1: Hydration (ChatService::build_context)

```rust
let context = ChatService::build_context(&mut conn, workspace_id, chat_file_id).await?;
// Returns BuiltContext { persona, history, attachments }
```

1. **Load Messages**: Fetch all messages for this chat from the database
2. **Extract Persona**: Use default or load from agent config
3. **Split History**: Exclude last message (the current prompt)
4. **Hydrate Attachments**: For each file ID in the last message's metadata:
   - Fetch file content from database
   - Verify workspace ownership (security)
   - Store in `attachments` vector

### Phase 2: Agent Creation (ChatActor::process_interaction)

```rust
let agent = rig_service
    .create_agent(pool, workspace_id, user_id, &session)
    .await?;
```

The agent is configured with:
- **System prompt**: From `context.persona`
- **Tools**: All 6 workspace tools (ls, read, write, rm, mv, touch)
- **Model**: GPT-4o-mini (configurable per chat)

### Phase 3: Streaming with Context

```rust
// Format attachments into the prompt
let attachments_context = if !context.attachments.is_empty() {
    let blocks: Vec<String> = context.attachments.iter()
        .map(|att| format!("File: {}\n---\n{}\n---", att.path, att.content))
        .collect();
    format!("\n\nAttached Files:\n{}", blocks.join("\n\n"))
} else {
    String::new()
};

// Combine current message + attachments
let prompt = format!("{}{}", last_message.content, attachments_context);

// Convert history to Rig format
let history = rig_service.convert_history(&context.history);

// Stream with full context
let mut stream = agent.stream_chat(&prompt, history).await;
```

### Phase 4: Tool Execution & Response

The AI receives:
- **System prompt**: "You are BuildScale AI, a professional software engineering assistant."
- **History**: Previous conversation turns
- **Current prompt**: User message + formatted file attachments
- **Tools**: Workspace file operations

The AI can then:
1. **Read** attached files to understand context
2. **Call tools** (ls, read, write, rm, mv, touch) to manipulate files
3. **Respond** with text or continue with more tool calls

## 4. Security: Workspace Isolation

The context builder enforces **workspace isolation** at hydration time:

```rust
if file_with_content.file.workspace_id == workspace_id {
    attachments.push(FileAttachment { ... });
}
// Files from other workspaces are silently ignored
```

This prevents:
- **Cross-workspace leakage**: AI can't see files from other workspaces
- **Token theft**: Malicious users can't attach files they don't own
- **Data exfiltration**: Attachments are validated against workspace membership

## 5. Future Enhancements (TODO)

### Token Limit Optimization

Currently, all context is passed to the AI without pruning. Future enhancements:

```rust
// TODO: Implement smart history truncation based on token limits
if context.estimate_tokens() > DEFAULT_CONTEXT_TOKEN_LIMIT {
    // Prune old messages from history
    // Summarize truncated history
    // Keep essential attachments only
}
```

### Priority-Based Fragment Management

Inspired by the original ContextMap vision, we could add:

- **Attachment priorities**: User can mark files as "essential" vs "optional"
- **Smart summarization**: Old conversation turns summarized to save tokens
- **Dynamic pruning**: Drop low-priority context when approaching limits

### Agent & Skill as Files

Because "Everything is a File", we could store:

- **Agent personas**: As `.agent` files in the workspace
- **Skill definitions**: As `.skill` files with JSON schemas
- **Context templates**: Reusable prompt snippets

This would allow:
- **Edit personas in UI**: Treat agent instructions like documents
- **Version control skills**: Track tool definition changes in git
- **Zero special cases**: All AI metadata is just files

## 6. Why "Structured Context" Beats "String Concatenation"

### Before (String Concatenation)
```rust
let context = format!(
    "System: {}\n\nHistory: {}\n\nFiles: {}\n\nUser: {}",
    persona, history_str, files_str, user_msg
);
// ❌ No type safety
// ❌ Hard to modify specific parts
// ❌ Parsing issues with XML markers
// ❌ Can't leverage framework features
```

### After (Structured Context)
```rust
let BuiltContext { persona, history, attachments } = build_context(...).await?;

let agent = Agent::builder()
    .with_system_prompt(&persona)
    .build();

let stream = agent.stream_chat(&prompt, history).await;
// ✅ Type safe
// ✅ Framework handles formatting
// ✅ Easy to extend with new fields
// ✅ Leverages Rig's native capabilities
```

## 7. Testing Strategy

The build_context tests verify:

1. **Persona presence**: `context.persona.contains("BuildScale AI")`
2. **History structure**: `context.history.len() == expected_count`
3. **Attachment content**: `context.attachments[0].content.contains(...)`
4. **Workspace isolation**: Files from other workspaces are excluded
5. **Empty chat handling**: Returns empty history/attachments but valid persona
6. **Large context**: Structure is maintained even with many messages/files

All tests use the structured fields directly - no string parsing needed!
