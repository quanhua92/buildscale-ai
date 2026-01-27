# Context Engineering: The Cortex Guide

BuildScale.ai treats LLM context as a dynamically engineered resource, managed through a structured **BuiltContext** architecture with separate managers for attachments and history. This replaces traditional string concatenation with a properly integrated, multi-modal context system that leverages the Rig.rs AI framework's native capabilities.

## 1. The Core Philosophy

The "Workspace is the OS" vision requires that the Agentic Engine can precisely control what an LLM "sees" at any given moment. By using **structured context** with **dedicated managers**, we achieve:

- **Framework Integration**: Persona, history, and attachments are passed to Rig in their native formats.
- **Type Safety**: No more string parsing or XML marker escaping issues.
- **Separation of Concerns**: Each manager has a clear responsibility
- **Graceful Degradation**: **Implemented** - Intelligent pruning when hitting token limits via priority-based fragment management.

## 2. Architecture: BuiltContext with Dedicated Managers

### The BuiltContext Struct

```rust
pub struct BuiltContext {
    /// System persona/instructions for the AI
    pub persona: String,
    /// History manager for conversation messages
    pub history: HistoryManager,
    /// Attachment manager for file attachments with priority-based pruning
    pub attachment_manager: AttachmentManager,
}
```

### AttachmentManager - File Attachments

Manages workspace file attachments with priority-based pruning:

```rust
pub struct AttachmentManager {
    pub map: IndexMap<AttachmentKey, AttachmentValue>,
}

pub enum AttachmentKey {
    WorkspaceFile(Uuid),  // Currently used for file attachments
    // Other variants reserved for future use
}

pub struct AttachmentValue {
    pub content: String,
    pub priority: i32,      // Higher = dropped first during pruning
    pub tokens: usize,      // Estimated token count
    pub is_essential: bool, // Never prune if true
}
```

**Features:**
- ✅ Token estimation using `ESTIMATED_CHARS_PER_TOKEN` (4 chars/token)
- ✅ Priority-based pruning via `optimize_for_limit()`
- ✅ Keyed addressability via `WorkspaceFile(file_id)`
- ✅ XML rendering with `<file_context>` markers
- ✅ Positional sorting for consistent order

### HistoryManager - Conversation History

Manages conversation history with token estimation and future pruning:

```rust
pub struct HistoryManager {
    pub messages: Vec<ChatMessage>,
}

impl HistoryManager {
    pub fn new(messages: Vec<ChatMessage>) -> Self { ... }
    pub fn estimate_tokens(&self) -> usize { ... }
    pub fn len(&self) -> usize { ... }
    pub fn is_empty(&self) -> bool { ... }
}

// Implements Deref<Vec<ChatMessage>> for transparent Vec access
// Implements IntoIterator for iteration
```

**Features:**
- ✅ Token estimation via `estimate_tokens()`
- ✅ Transparent Vec access via Deref implementation
- ✅ Convenience methods: `len()`, `is_empty()`
- ✅ Iterator support via IntoIterator
- 🔄 **Future**: Sliding window, summarization, token-based pruning

### Priority Constants

```rust
// Pruning Priorities (Higher = dropped first)
pub const PRIORITY_ESSENTIAL: i32 = 0;  // Never dropped
pub const PRIORITY_HIGH: i32 = 3;       // Dropped last
pub const PRIORITY_MEDIUM: i32 = 5;     // User attachments (default)
pub const PRIORITY_LOW: i32 = 10;       // Dropped first
```

## 3. The Lifecycle: From IDs to AI

### Phase 1: Hydration (ChatService::build_context)

```rust
let context = ChatService::build_context(&mut conn, workspace_id, chat_file_id).await?;
// Returns BuiltContext { persona, history, attachment_manager }
```

1. **Load Messages**: Fetch all messages for this chat from the database
2. **Extract Persona**: Use the high-intelligence "Coworker" persona from the `agents` registry or load from the chat's persistent agent config.
3. **Split History**: Exclude last message (the current prompt), wrap in `HistoryManager`
4. **Hydrate Attachments** using `AttachmentManager`:
   - Fetch file content from database
   - Verify workspace ownership (security)
   - **Estimate tokens**: `content.len() / ESTIMATED_CHARS_PER_TOKEN`
   - **Add to AttachmentManager**: `AttachmentKey::WorkspaceFile(file_id)` with `PRIORITY_MEDIUM`
5. **Optimize Attachments**: Call `attachment_manager.optimize_for_limit(DEFAULT_CONTEXT_TOKEN_LIMIT)`
6. **Sort Attachments**: Call `attachment_manager.sort_by_position()` for consistent rendering order

### Phase 2: Agent Creation (ChatActor::process_interaction)

```rust
let agent = rig_service
    .create_agent(pool, workspace_id, user_id, &session)
    .await?;
```

The agent is configured with:
- **System prompt**: From `context.persona`
- **Tools**: All workspace tools (ls, read, write, rm, mv, touch, edit, grep)
- **Model**: GPT-4o-mini (configurable per chat)

### Phase 3: Streaming with Context

```rust
// Format attachments from AttachmentManager (with XML markers)
let attachments_context = if !context.attachment_manager.map.is_empty() {
    context.attachment_manager.render()  // <file_context>...</file_context>
} else {
    String::new()
};

// Combine current message + attachments
let prompt = format!("{}{}", last_message.content, attachments_context);

// Convert history to Rig format (via HistoryManager)
let history = rig_service.convert_history(&context.history.messages);

// Stream with full context
let mut stream = agent.stream_chat(&prompt, history).await;
```

### Phase 4: Tool Execution & Response

The AI receives:
- **System prompt**: The "Coworker" persona which mandates an **"Explore First"** protocol.
- **History**: Previous conversation turns (via `HistoryManager`)
- **Current prompt**: User message + formatted file attachments (via `AttachmentManager`)
- **Tools**: Workspace file operations (ls, read, write, rm, mv, touch, edit, grep)

The AI is instructed to:
1. **Explore First**: Never guess workspace structure. Always use `ls` or `grep` to map context before answering.
2. **Read** attached or discovered files to understand implementation details.
3. **Call tools** to manipulate files, preferring `edit` for precision.
4. **Respond** with text or continue with more tool calls, using `<thinking>` blocks for complex plans.

## 4. Security: Workspace Isolation

The context builder enforces **workspace isolation** at hydration time:

```rust
if file_with_content.file.workspace_id == workspace_id {
    attachment_manager.add_fragment(
        AttachmentKey::WorkspaceFile(*file_id),
        AttachmentValue { ... }
    );
}
// Files from other workspaces are silently ignored
```

This prevents:
- **Cross-workspace leakage**: AI can't see files from other workspaces
- **Token theft**: Malicious users can't attach files they don't own
- **Data exfiltration**: Attachments are validated against workspace membership

## 5. Token Limit Optimization (Implemented)

### Automatic Pruning with AttachmentManager

The `AttachmentManager::optimize_for_limit()` method implements intelligent pruning:

```rust
pub fn optimize_for_limit(&mut self, max_tokens: usize) {
    let current_tokens: usize = self.map.values().map(|v| v.tokens).sum();

    if current_tokens <= max_tokens {
        return;  // No pruning needed
    }

    // Sort non-essential fragments by priority (descending)
    let mut candidates: Vec<(AttachmentKey, i32)> = self.map
        .iter()
        .filter(|(_, v)| !v.is_essential)
        .map(|(k, v)| (k.clone(), v.priority))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove fragments until under the limit
    for (key, _) in candidates {
        if let Some(value) = self.map.get(&key) {
            current_tokens -= value.tokens;
            self.map.shift_remove(&key);
            if current_tokens <= max_tokens {
                break;
            }
        }
    }
}
```

### How It Works

1. **Token Estimation**: Each fragment estimates tokens using `content.len() / 4` (chars per token)
2. **Priority Check**: Essential fragments (`is_essential = true`) are never pruned
3. **Sorted Removal**: Non-essential fragments removed from lowest to highest priority
4. **Early Exit**: Stops as soon as we're under the token limit

### Current Behavior

- **User-attached files**: Marked with `PRIORITY_MEDIUM` (5)
- **No essential files**: Currently all files are prunable
- **Default limit**: `DEFAULT_CONTEXT_TOKEN_LIMIT = 4000` tokens
- **History pruning**: Not yet implemented (future enhancement)

## 6. Future Enhancements (TODO)

### History Pruning with HistoryManager

Currently, `HistoryManager` only wraps messages and provides token estimation. Future enhancements:

```rust
impl HistoryManager {
    // Future: Sliding window - keep only last N messages
    pub fn truncate_to_max_messages(&mut self, max: usize) { ... }

    // Future: Token-based pruning
    pub fn prune_to_limit(&mut self, max_tokens: usize) { ... }

    // Future: Smart summarization
    pub fn summarize_old_messages(&mut self) { ... }
}
```

### Essential File Marking

Allow users to mark files as "essential" to prevent pruning:

```rust
attachment_manager.add_fragment(
    AttachmentKey::WorkspaceFile(file_id),
    AttachmentValue {
        content,
        priority: PRIORITY_HIGH,
        tokens: estimated_tokens,
        is_essential: true,  // Never prune this file
    },
);
```

### Persona Manager

Could extract persona into its own manager:

```rust
pub struct PersonaManager {
    pub persona: String,
    // Future: Multiple persona sources
    // - System default
    // - Workspace-specific
    // - User-defined
}
```

### Dynamic Priority Adjustment

Allow AI to suggest priority adjustments based on relevance:
- AI analyzes file content and user query
- Automatically boosts priority of relevant files
- Lowers priority of irrelevant context

## 7. Why "Managers" Beat "Simple Structs"

### Before (Simple Structs)
```rust
pub struct BuiltContext {
    pub persona: String,
    pub history: Vec<ChatMessage>,  // ❌ No token estimation, no pruning
    pub attachments: Vec<FileAttachment>,  // ❌ No pruning, no priorities
}
```

**Limitations:**
- No token limit optimization
- No priority system
- Can't selectively prune files or messages
- No keyed addressability
- No token estimation

### After (Dedicated Managers)
```rust
pub struct BuiltContext {
    pub persona: String,
    pub history: HistoryManager,       // ✅ Token estimation, future pruning
    pub attachment_manager: AttachmentManager,  // ✅ Pruning, priorities, tokens
}
```

**Advantages:**
- ✅ **Separation of Concerns**: Each manager has a clear responsibility
- ✅ **AttachmentManager**: Automatic pruning, priority system, token estimation
- ✅ **HistoryManager**: Token estimation, Deref to Vec, ready for pruning
- ✅ **Extensibility**: Easy to add PersonaManager, SkillManager, etc.
- ✅ **Type Safety**: Structured access through manager methods

## 8. Testing Strategy

The build_context tests verify:

1. **Persona presence**: `context.persona.contains("BuildScale AI")`
2. **History structure**: `context.history.len() == expected_count` (via Deref)
3. **History tokens**: `context.history.estimate_tokens()` (future usage)
4. **Attachment count**: `context.attachment_manager.map.len() == expected_count`
5. **Attachment content**: Extract from `context.attachment_manager.map.values()`
6. **Workspace isolation**: Files from other workspaces excluded (empty `attachment_manager.map`)
7. **Empty chat handling**: Empty history/attachments but valid persona
8. **Token optimization**: `AttachmentManager` prunes attachments when over limit
9. **Key types**: Verify attachments use `AttachmentKey::WorkspaceFile(_)`

### Example Test Assertions

```rust
// HistoryManager (transparent Vec access)
assert!(!context.history.is_empty());
assert_eq!(context.history.len(), 2);
assert_eq!(context.history[0].content, "Hello");

// HistoryManager token estimation
let history_tokens = context.history.estimate_tokens();

// AttachmentManager
assert!(!context.attachment_manager.map.is_empty());
assert_eq!(context.attachment_manager.map.len(), 1);

// Extract file content
let file_content = context.attachment_manager.map
    .values()
    .next()
    .expect("Should have one attachment");
assert!(file_content.content.contains("Hello World"));

// Verify token optimization
let total_tokens: usize = context.attachment_manager.map
    .values()
    .map(|v| v.tokens)
    .sum();
assert!(total_tokens < DEFAULT_CONTEXT_TOKEN_LIMIT * 2);
```

All tests use the structured managers directly - no string parsing needed!
