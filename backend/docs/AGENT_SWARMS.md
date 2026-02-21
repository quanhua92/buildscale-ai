← [Back to Index](./README.md) | **Developer API**: [REST API Guide](./REST_API_GUIDE.md)

# Agent Swarms & Sessions

## Overview

BuildScale provides a comprehensive agent session management system that enables tracking, monitoring, and controlling AI agent processes. This system is designed to scale from single-server deployments to distributed agent swarms across multiple servers.

## Current Implementation

### Database-Based Session Tracking

Agent sessions are persisted in the `agent_sessions` table, providing:

- **Visibility**: See which agents are running in your workspace
- **Control**: Pause, resume, or cancel agent sessions
- **Persistence**: Sessions survive server restarts
- **Monitoring**: Track agent status, current task, and heartbeat

### Session Lifecycle

```
┌─────────┐     ┌──────────┐     ┌────────┐     ┌───────────┐     ┌───────────┐     ┌───────────┐
│  Create │ ──> │   Idle   │ ──> │ Running│ ──> │ Completed │     │  Cancelled │     │   Error   │
└─────────┘     └──────────┘     └────────┘     └───────────┘     └───────────┘     └───────────┘
                     │                │
                     ▼                ▼
                  ┌────────┐     ┌────────┐
                  │ Paused │     │ Error  │
                  └────────┘     └────────┘
```

### Session States

| State | Description | Transitions From |
|-------|-------------|------------------|
| `idle` | Agent is ready but not processing | Create, Running, Paused |
| `running` | Agent is actively processing | Idle, Paused |
| `paused` | Agent is temporarily paused | Running, Idle |
| `completed` | Agent finished successfully | Running, Paused |
| `cancelled` | Agent was cancelled by user | Running, Idle, Paused |
| `error` | Agent encountered an error | Running |

### Heartbeat Mechanism

- **Frequency**: Every 30 seconds while running
- **Threshold**: Sessions with heartbeat > 120 seconds are considered stale
- **Cleanup**: Automatic cleanup of stale sessions (excluding completed/error states)

## Agent Types

### Assistant
- **Purpose**: General-purpose conversational AI
- **Mode**: `chat`
- **Use Case**: Answering questions, providing explanations, casual interactions

### Planner
- **Purpose**: Strategic planning and task breakdown
- **Mode**: `plan`
- **Use Case**: Creating implementation plans, breaking down complex tasks

### Builder
- **Purpose**: Code generation and file manipulation
- **Mode**: `build`
- **Use Case**: Writing code, modifying files, implementing features

## API Endpoints

### Workspace Scope
- `GET /api/v1/workspaces/:id/agent-sessions` - List all active sessions in workspace

### Global Scope (Session Owner Required)
- `GET /api/v1/agent-sessions/:id` - Get session details
- `POST /api/v1/agent-sessions/:id/pause` - Pause session
- `POST /api/v1/agent-sessions/:id/resume` - Resume session
- `DELETE /api/v1/agent-sessions/:id` - Cancel session

See [REST API Guide](./REST_API_GUIDE.md#agent-sessions-api) for detailed API documentation.

## ChatActor Integration

The `ChatActor` automatically manages session lifecycle:

1. **Session Creation**: When a chat actor starts, it creates a session record
2. **Status Updates**: Status changes are persisted (idle → running → completed/cancelled)
3. **Heartbeat**: Automatic heartbeat every 30 seconds while running
4. **Shutdown**: Session persists in last state (idle/running/cancelled)

```rust
// Session creation in ChatActor
let session = create_session(
    &mut conn,
    workspace_id,
    chat_id,
    user_id,
    AgentType::Assistant,
    model.clone(),
    mode.clone(),
).await?;

// Status update on state change
update_session_status(&mut conn, session_id, SessionStatus::Running, user_id).await?;

// Heartbeat while running
start_heartbeat_task(session_id, pool.clone());

// Shutdown - session persists in last state (no longer marks as completed)
// Stale sessions will be handled by cleanup worker if not re-activated
```

## Future: Distributed Agent Swarms

### Architecture Vision

The current database-based session tracking is the foundation for distributed agent swarms:

```
┌─────────────────────────────────────────────────────────────┐
│                    Load Balancer / API Gateway              │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   Server 1  │      │   Server 2  │      │   Server 3  │
│             │      │             │      │             │
│ ChatActors  │      │ ChatActors  │      │ ChatActors  │
│ ├─ Actor A  │      │ ├─ Actor D  │      │ ├─ Actor G  │
│ ├─ Actor B  │      │ ├─ Actor E  │      │ └─ Actor H  │
│ └─ Actor C  │      │ └─ Actor F  │      │             │
└─────────────┘      └─────────────┘      └─────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              ▼
                    ┌─────────────────┐
                    │   PostgreSQL    │
                    │                 │
                    │ agent_sessions  │
                    │ - Global state  │
                    │ - Coordination  │
                    │ - Discovery     │
                    └─────────────────┘
```

### Key Features for Distributed Swarms

#### 1. Global Session Registry
- All servers write to the same `agent_sessions` table
- Enables cross-server session discovery and coordination
- Prevents duplicate sessions (chat_id uniqueness constraint)

#### 2. Server Assignment
```sql
ALTER TABLE agent_sessions ADD COLUMN server_id TEXT;
ALTER TABLE agent_sessions ADD COLUMN server_hostname TEXT;
CREATE INDEX idx_agent_sessions_server_id ON agent_sessions(server_id);
```

#### 3. Health Monitoring
```sql
-- Sessions from crashed servers are automatically detected
SELECT * FROM agent_sessions
WHERE last_heartbeat < NOW() - INTERVAL '30 seconds'
AND server_id = 'crashed-server-id';
```

#### 4. Session Migration
- Transfer sessions between servers (load balancing)
- Session handoff during server maintenance
- Automatic rebalancing based on server load

#### 5. Inter-Agent Communication
- Agents can discover and communicate with other agents
- Swarm coordination patterns (master-worker, pipeline, MapReduce)
- Event-driven agent collaboration

### Implementation Roadmap

#### Phase 1: Foundation (Current)
- ✅ Database-based session tracking
- ✅ Session lifecycle management
- ✅ Heartbeat and cleanup
- ✅ REST API for session control

#### Phase 2: Multi-Server Support
- [ ] Server assignment and tracking
- [ ] Cross-server session discovery
- [ ] Health monitoring across servers
- [ ] Graceful shutdown with session migration

#### Phase 3: Advanced Coordination
- [ ] Inter-agent messaging system
- [ ] Swarm orchestration patterns
- [ ] Dynamic load balancing
- [ ] Fault tolerance and recovery

#### Phase 4: Swarm Intelligence
- [ ] Multi-agent collaboration
- [ ] Task distribution and aggregation
- [ ] Emergent behavior patterns
- [ ] Self-organizing swarms

## Usage Examples

### List Active Agents in Workspace

```typescript
const sessions = await apiClient.getWorkspaceAgentSessions(workspaceId);
console.log(`Active agents: ${sessions.total}`);
sessions.sessions.forEach(session => {
  console.log(`- ${session.agent_type}: ${session.current_task}`);
});
```

### Monitor Agent Status

```typescript
// Poll for status updates every 5 seconds
const interval = setInterval(async () => {
  const session = await apiClient.getAgentSession(sessionId);
  console.log(`Status: ${session.status}, Task: ${session.current_task}`);

  if (session.status === 'completed' || session.status === 'error' || session.status === 'cancelled') {
    clearInterval(interval);
  }
}, 5000);
```

### Control Agent Session

```typescript
// Pause a running agent
await apiClient.pauseAgentSession(sessionId, { reason: 'User intervention' });

// Resume with new task
await apiClient.resumeAgentSession(sessionId, { task: 'Continue with optimization' });

// Cancel if no longer needed
await apiClient.cancelAgentSession(sessionId);
```

### SSE Event Streaming

For real-time updates, use the existing SSE event stream:

```typescript
const eventSource = new EventSource(`/api/v1/workspaces/${workspaceId}/chats/${chatId}/events`);

eventSource.addEventListener('agent_status', (event) => {
  const data = JSON.parse(event.data);
  console.log(`Agent ${data.agent_type} is now ${data.status}`);
});

eventSource.addEventListener('agent_task', (event) => {
  const data = JSON.parse(event.data);
  console.log(`Task: ${data.task}`);
});
```

## Best Practices

### 1. Session Cleanup
- Cancel sessions when no longer needed (sets status to Cancelled, preserves history)
- Don't rely solely on automatic cleanup
- Implement proper shutdown in your applications

### 2. Heartbeat Monitoring
- Monitor heartbeat failures for early detection of issues
- Set up alerts for stale sessions
- Use heartbeat patterns to detect server crashes

### 3. Error Handling
- Handle `completed` and `error` terminal states gracefully
- Implement retry logic for transient failures
- Log session state transitions for debugging

### 4. Resource Management
- Limit concurrent sessions per workspace
- Implement session timeouts for inactive agents
- Monitor resource usage across all active sessions

## Troubleshooting

### Sessions Not Appearing in List
- Verify user is a workspace member
- Check session status (completed/error sessions may be filtered)
- Ensure session heartbeat is recent (< 120 seconds)

### Cannot Pause/Resume Session
- Verify you are the session owner
- Check current session status (can only pause running sessions)
- Ensure session is not already in terminal state

### Stale Sessions Not Cleaned Up
- Verify cleanup job is running
- Check session status (completed/error sessions are not cleaned up)
- Adjust STALE_SESSION_THRESHOLD_SECONDS if needed

## Related Documentation

- [REST API Guide](./REST_API_GUIDE.md) - Complete API reference
- [Agentic Engine](./AGENTIC_ENGINE.md) - Agent architecture details
- [RIG Integration](./RIG_INTEGRATION.md) - AI provider integration
- [Chat Persistence](./CHAT_PERSISTENCE_AUDIT.md) - Chat state management
