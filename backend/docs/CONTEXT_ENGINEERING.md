# Context Engineering: The Cortex Guide

BuildScale.ai treats LLM context as a dynamically engineered resource, managed through a structured **Context Map**. This replaces traditional string concatenation with an addressable, prioritized registry of information fragments.

## 1. The Core Philosophy

The "Workspace is the OS" vision requires that the Agentic Engine can precisely control what an LLM "sees" at any given moment. By using an **Ordered Map** (`IndexMap<ContextKey, ContextValue>`), we achieve:

- **Keyed Addressability**: Replace or update specific context blocks (e.g., an updated file) without rebuilding the whole prompt.
- **Positional Integrity**: Ensure the LLM receives information in a logical order (System -> Workspace -> History -> Request).
- **Graceful Degradation**: Intelligent pruning of low-priority info when hitting token limits.

## 2. Anatomy of the Context Map

### ContextKey (The "Where")
Defines the category and identity of a fragment.

| Key | Description | Positional Order |
| :--- | :--- | :--- |
| `SystemPersona` | The core instructions from the `.agent` file. | 0 (First) |
| `ActiveSkill(id)` | JSON Schema for a specific `.skill` tool. | 1 |
| `WorkspaceFile(id)` | Literal content of an attached file. | 2 |
| `Environment` | Metadata (CWD, active branch, OS info). | 3 |
| `ChatHistory` | Previous messages in the session. | 4 |
| `UserRequest` | The current prompt from the user. | 5 (Last) |

### ContextValue (The "What")
Stores the content and engineering metadata.

- **Content**: The raw text or JSON string.
- **Priority**: A weight used for pruning (Higher = more likely to be dropped).
- **Tokens**: Cached count for limit management.
- **Is Essential**: A flag that prevents the fragment from ever being pruned (e.g., the User's question).

## 3. The Lifecycle: From ID to Prompt

1. **Hydration**: The `ChatService` takes message metadata (IDs) and uses `FilesService` to read the literal bytes from the NVMe mirror or DB.
2. **Fragmentation**: Bytes are wrapped in XML-style markers (e.g., `<file path="...">...</file>`) and inserted into the `ContextMap`.
3. **Optimization (The "Pruning")**: 
   - If `total_tokens > model_limit`:
   - Sort non-essential fragments by priority.
   - Remove fragments until `total_tokens <= budget`.
4. **Rendering**: The `ContextMap` is flattened into a single string for the LLM Provider (OpenAI/Anthropic).

## 4. Why "Everything is a File" Matters

Because Agents and Skills are stored as files in the registry, the Context Engine can simply "read" them using the standard `FilesService`. This means:
- You can **edit** your agent's persona in the UI like a document.
- You can **version-control** your skills.
- The Agentic Engine has zero "special case" storage logic for AI metadata.
