# Chat Persistence & Audit Trail Specification

## 1. Problem Statement

Current system loses critical audit information during streaming interactions:

- **AI reasoning/thinking chunks** (`Thought` SSE events) - only streamed, not persisted
- **Tool invocations** (`Call` SSE events) with full arguments - only streamed, not persisted
- **Tool execution results** (`Observation` SSE events) - only streamed, not persisted

Only the final Assistant response and a summarized tool actions log are saved to the database.

**Impact**: When reopening a `.chat` file, users only see human messages and final AI responses. The complete interaction trail—including why the AI made decisions, what tools it called, and what those tools returned—is permanently lost. This cripples debugging, compliance, and understanding of AI behavior.

---

## 2. Current Architecture (Pre-Fix)

### 2.1 Stream Event Flow

```
Rig Stream → Actor → SSE (frontend) [NO PERSISTENCE]
    │
    ├─ Thought      → SseEvent::Thought      → ❌ Lost
    ├─ ToolCall     → SseEvent::Call         → ❌ Lost
    ├─ ToolResult   → SseEvent::Observation  → ❌ Lost
    └─ Text Chunk   → SseEvent::Chunk        → ✅ Aggregated into final message
```

### 2.2 What Gets Saved (Database)

**Persisted:**
- User messages ✅
- Final Assistant response ✅
- System markers (cancellation) ✅
- Aggregated tool action summary (text-only, limited to file modifications) ⚠️

**Lost:**
- All reasoning chunks (except what's implicitly in final response)
- All tool calls (arguments JSON)
- All tool results (outputs, success/failure)
- Non-file-modification tools (ls, grep, read, ask_user, exit_plan_mode) have NO persistence

### 2.3 Schema Capability

The `chat_messages` table and `ChatMessage` model already support:

```rust
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
    Tool,  // ← Defined but never used
}

pub struct ChatMessageMetadata {
    pub attachments: Vec<ChatAttachment>,
    pub tool_calls: Option<serde_json::Value>,  // ← Defined but never populated
    pub usage: Option<serde_json::Value>,
    pub model: Option<String>,
    pub response_id: Option<String>,
    pub question_answer: Option<QuestionAnswerMetadata>,
}
```

The schema is ready—we just need to use it.

---

## 3. Proposed Solution: Full Event Persistence

### 3.1 Design Principles

1. **Every SSE event = persistent ChatMessage** (with appropriate role & metadata)
2. **No data loss**: All streaming content saved with millisecond precision
3. **Queryable audit trail**: Can reconstruct exact interaction timeline via simple query
4. **Backward compatible**: Existing messages remain valid, old chats still load
5. **Frontend control**: UI decides which events to render, collapse, or hide
6. **Minimal schema changes**: Reuse existing fields, add only optional metadata

### 3.2 Message Type Mapping

| Stream Item | ChatMessageRole | `message_type` | Key Metadata Fields |
|-------------|----------------|----------------|---------------------|
| Buffered Reasoning | `Assistant` | `reasoning_complete` | `reasoning_id` (UUID) |

**Note**: Reasoning chunks are buffered in memory and saved as a single aggregated `ChatMessage` with `message_type="reasoning_complete"` whenever the agent transitions to a new content type (tool call, final response) or finishes the turn. This avoids database flooding and simplifies frontend rendering. Individual `ReasoningDelta` chunks are still streamed via SSE for real-time responsiveness.

### 3.3 Implementation Requirements

#### 3.3.1 Schema Extension

**No database migration needed**—all new data stored in JSONB metadata.

Extend `ChatMessageMetadata` (in `src/models/chat.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessageMetadata {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_success: Option<bool>,
}
```

#### 3.3.2 Persistence Layer

Add `save_stream_event` method to `ChatService` (`src/services/chat/mod.rs`):

```rust
pub async fn save_stream_event(
    conn: &mut DbConn,
    storage: &FileStorageService,
    workspace_id: Uuid,
    file_id: Uuid,
    role: ChatMessageRole,
    content: String,
    metadata: ChatMessageMetadata,
) -> Result<ChatMessage> {
    self.save_message(conn, storage, workspace_id, NewChatMessage {
        file_id,
        workspace_id,
        role,
        content,
        metadata: sqlx::types::Json(metadata),
    }).await
}
```

#### 3.3.3 Actor Modifications

**File**: `src/services/chat/actor.rs`

**A. Extend state** (`ChatActorState`):

```rust
struct ChatActorState {
    // ... existing fields ...
    tool_actions_log: Vec<String>,
    current_reasoning_id: Option<String>,  // UUID for this turn's reasoning
    reasoning_buffer: Vec<String>,         // Buffer for streaming reasoning chunks
}
```

Update `Default` to include empty buffer.

**B. Modify `process_stream_item` to handle buffering**:

- `ReasoningDelta`: Append to `reasoning_buffer`, stream to SSE. **No DB save**.
- `Reasoning` (final): Append all parts to `reasoning_buffer`, stream to SSE, then call `flush_reasoning_buffer()`.
- `ToolCall`, `Text`, `FinalResponse`: Call `flush_reasoning_buffer()` before processing the event.

**C. Helper Method: `flush_reasoning_buffer`**:

Concatenates `reasoning_buffer` into a single string and saves it as a `ChatMessage` with `role=Assistant` and `message_type="reasoning_complete"`. Clears the buffer after saving.

**D. Cleanup**:

After final response is saved, clear `current_reasoning_id` to prepare for next turn.

**E. Remove Aggregated Tool Log**:

Optionally remove the `tool_actions_log` accumulation and its save block (lines 393-422) since individual tool results are now persisted. **Keep it temporarily** for backward compatibility, but can remove after frontend transition.

### 3.4 Data Minimization

To prevent database bloat, tool inputs and outputs are summarized before persistence:

- **Tool Outputs (`tool_output`)**:
  - `read`: Truncated with stats (bytes, lines) and preview.
  - `ls`, `grep`: Limit number of items/matches shown.
  - `default`: Generic size cap (e.g. 2KB).
- **Tool Inputs (`tool_arguments`)**:
  - `write`: Content field truncated if > 1KB.
  - `edit`: Diff fields truncated if > 500 chars.

This ensures essential audit info is preserved without storing megabytes of raw data.

---

## 4. History Reconstruction & Context

### 4.1 AI Context (Unchanged)

`convert_history()` continues to exclude `Tool` and `System` messages. The AI should **not** see raw tool calls/results in its context—those are managed by Rig's runtime. Only `User` and `Assistant` messages are sent to the LLM in subsequent turns.

```rust
// Existing behavior preserved
Message::user(msg.content.clone())  // User
Message::assistant(msg.content.clone())  // Assistant
_ => None,  // System, Tool excluded
```

### 4.2 Audit Trail Query

Full timeline reconstruction:
```sql
SELECT
    role,
    content,
    metadata->>'message_type' as message_type,
    metadata->>'reasoning_id' as reasoning_id,
    metadata->>'tool_name' as tool_name,
    metadata->>'tool_arguments' as tool_arguments,
    metadata->>'tool_output' as tool_output,
    metadata->>'tool_success' as tool_success,
    created_at
FROM chat_messages
WHERE file_id = $1
ORDER BY created_at ASC;
```

### 4.3 Frontend Filtering

Frontend can choose to:
- Show all messages (debug/audit mode)
- Hide `Tool` and `reasoning_chunk` messages (default user mode)
- Collapse consecutive `reasoning_chunk` messages into expandable "AI thinking..." block

---

## 5. Testing Strategy

### 5.1 Unit Tests

**File**: `tests/services/chat/persistence_tests.rs`

```rust
#[tokio::test]
async fn test_save_reasoning_chunk_creates_message_with_message_type() { ... }

#[tokio::test]
async fn test_reasoning_chunks_share_same_reasoning_id() { ... }

#[tokio::test]
async fn test_save_tool_call_saves_tool_role_with_metadata() { ... }

#[tokio::test]
async fn test_save_tool_result_links_to_tool_name() { ... }

#[tokio::test]
async fn test_tool_result_matches_previous_call() { ... }
```

### 5.2 Integration Tests

**File**: `tests/services/chat/integration_audit_tests.rs`

```rust
#[tokio::test]
async fn test_full_interaction_persists_all_events_in_order() {
    // Mock LLM that streams:
    // 1. ReasoningDelta chunks
    // 2. ToolCall (ls)
    // 3. ToolResult
    // 4. Final text response
    // Verify all saved to DB with correct metadata and ordering
}
```

### 5.3 Existing Test Updates

- `actor_tests.rs` - May need adjustments if tool_actions_log is removed
- `rig_engine_tests.rs` - Should pass unchanged

---

## 6. Frontend Integration

### 6.1 TypeScript Types

Update chat message interface:

```typescript
interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  message_type?: 'reasoning_chunk' | 'reasoning_complete' | 'tool_call' | 'tool_result';
  metadata: {
    reasoning_id?: string;
    tool_name?: string;
    tool_arguments?: Record<string, any>;
    tool_output?: string;
    tool_success?: boolean;
    // ... existing fields
  };
  created_at: string;
}
```

### 6.2 Rendering Logic

**Message Component**:

- `reasoning_chunk`: Render inside collapsible `<details>` with `reasoning_id` grouping
- `tool_call`: Show with tool icon, expandable to show JSON arguments
- `tool_result`: Indented, color-coded border (green=success, red=failure)
- `tool` + `reasoning_complete`: Minimal marker

**Toggle Option** (future): "Show Technical Details" checkbox to filter/hide Tool and reasoning_chunk messages.

### 6.3 No API Changes

The existing chat messages API endpoint returns all messages. Frontend automatically receives new message types after backend deployment.

---

## 7. Migration & Rollout

### 7.1 Zero-Downtime Deployment

1. Deploy backend changes (new metadata fields, persistence logic)
2. Old chats: Missing `message_type` = normal messages (backward compatible)
3. New chats: All events persisted automatically
4. Frontend: Initially filter out new types, gradually enable rendering

### 7.2 Backfill

**Not feasible**: Reasoning and tool call data were never captured, so cannot backfill old chats. Optionally add synthetic marker messages like `"[Reasoning: not recorded (pre-audit version)]"` but not recommended.

### 7.3 Rollback

- Backend: Revert commit, redeploy (old code ignores new metadata)
- Frontend: Continue filtering out unknown `message_type` values

---

## 8. Monitoring & Alerting

### 8.1 Metrics to Track

- **Messages per turn**: Median, p95, p99 (spike detection)
- **Storage growth**: Daily MB increase in `chat_messages`
- **Message type distribution**: Ratio of Tool to Assistant messages
- **Reasoning chunks per turn**: Average count

### 8.2 Alerts

- `>50 messages in single turn` → Possible infinite loop
- `>1MB chat file size` → Performance degradation risk
- `Persistence error rate > 0.1%` → Database issues

### 8.3 Logging

Enable debug logs for `save_stream_event` in development:
```rust
tracing::debug!(
    chat_id = %self.chat_id,
    message_type = ?metadata.message_type,
    reasoning_id = ?metadata.reasoning_id,
    "Persisted stream event"
);
```

---

## 9. Performance Considerations

### 9.1 Expected Message Bloat

- **Baseline**: 1-3 messages per turn (User, Assistant, optional System)
- **With audit trail**: 5-30 messages per turn (adds reasoning chunks, tool calls/results)
- **Estimate**: 3-10× increase in message count
- **Impact**: Minimal—storage is cheap, queries are indexed on `file_id + created_at`

### 9.2 Optimization (Future)

If load times degrade:
- Implement **virtual scrolling** in frontend
- Add **pagination** to chat messages API
- Batch insert tool results (currently one-by-one)
- Create **partial index** on `(file_id, (metadata->>'message_type'))` for filtered queries

---

## 10. Success Criteria

✅ All reasoning chunks persisted with shared `reasoning_id`  
✅ All tool calls saved as `ChatMessage` with `role=Tool` and full arguments  
✅ All tool results saved with `tool_name`, `tool_output`, `tool_success`  
✅ Full timeline reconstructable via `ORDER BY created_at`  
✅ No performance regression (p95 load time <200ms for 1000 messages)  
✅ Storage increase <10× (expected 3-5×)  
✅ 100% unit + integration test coverage for new code  
✅ Frontend renders new message types appropriately  
✅ Documentation complete (this spec + API guide updates)  

---

## 11. Open Questions & Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| Remove `tool_actions_log`? | **Yes** (after frontend transition) | Avoid duplication, new structured data is superior |
| Reasoning chunk granularity? | **Buffered & Aggregated** | Avoids DB flooding, simplifies UI rendering, reduces storage overhead |
| Frontend toggle for audit view? | **Phase 2** | Nice-to-have, not blocking v1 |
| Group tool call + result? | **No**—keep separate | Precise ordering, independent visibility, easier querying |
| Add `turn_number` field? | **No** (future optional) | Can derive from `created_at` or add later if needed |
| Add `parent_message_id`? | **No** (future optional) | Ordering sufficient for now, explicit linking adds complexity |

---

## 12. Implementation Phases

### Phase 1: Backend Core (Week 1)
- [ ] Extend `ChatMessageMetadata` with new optional fields
- [ ] Add `ChatService::save_stream_event`
- [ ] Inject `ChatService` into `ChatActor`
- [ ] Add `current_reasoning_id` to `ChatActorState`
- [ ] Modify `process_stream_item` for all 4 event types
- [ ] Clear reasoning_id on turn completion
- [ ] Unit tests for each persistence case
- [ ] Integration test for full interaction

### Phase 2: Frontend Integration (Week 2)
- [ ] Update TypeScript message types
- [ ] Render `reasoning_chunk` as collapsible block
- [ ] Render `tool` messages with appropriate styling/icons
- [ ] Implement "Show Technical Details" toggle (optional)
- [ ] Test with real interactions in staging

### Phase 3: Documentation & Monitoring (Week 3)
- [ ] Update `REST_API_GUIDE.md` with new message types
- [ ] Update `AGENTIC_ENGINE.md` with persistence guarantees
- [ ] Add monitoring dashboards (messages/turn, storage growth)
- [ ] Create audit query examples in `docs/AUDIT_QUERIES.md`
- [ ] Final QA and performance testing

### Phase 4: Cleanup & Optimization (Week 4+)
- [ ] Remove `tool_actions_log` accumulation (after confirming frontend doesn't need it)
- [ ] Consider batch inserts if performance requires
- [ ] Add partial indexes for common filtered queries
- [ ] Evaluate need for `turn_number` or `parent_message_id`

---

## 13. Rollout Plan

1. **Staging**: Deploy backend + frontend changes to staging environment
2. **Canary**: Enable for 10% of chats, monitor message count, errors
3. **Gradual**: Increase to 50%, then 100% over 1 week
4. **Monitor**: Watch metrics, storage growth, frontend performance
5. **Iterate**: Address any issues (slow queries, UI clutter)

---

## 14. References

- **Current Architecture**: `docs/AGENTIC_ENGINE.md`, `docs/RIG_INTEGRATION.md`
- **Schema**: `src/models/chat.rs`
- **Streaming Processor**: `src/services/chat/actor.rs`
- **Persistence Layer**: `src/services/chat/mod.rs`
- **Tool Bridge**: `src/services/chat/rig_tools.rs`
- **Provider Abstraction**: `src/providers/`, `src/services/chat/rig_engine.rs`

---

**Document Version**: 1.0  
**Status**: Approved for Implementation  
**Last Updated**: 2026-02-07
