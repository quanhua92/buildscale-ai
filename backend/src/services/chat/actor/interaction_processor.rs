//! Background interaction processing module.
//!
//! This module contains all the logic for processing AI interactions in a background task.
//! It extracts methods from ChatActor that need to run independently of the main event loop.

use crate::models::agent_session::AgentType;
use crate::models::chat::{ChatMessageMetadata, ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use crate::models::sse::SseEvent;
use crate::providers::Agent;
use crate::services::agent_sessions;
use crate::services::chat::ChatService;
use crate::services::chat::registry::AgentRegistry;
use crate::services::chat::rig_engine::RigService;
use crate::services::chat::states::SharedActorState;
use crate::services::storage::FileStorageService;
use crate::state::TagIndexMessage;
use crate::DbPool;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// Import constants from the actor module
use super::constants::STREAM_READ_TIMEOUT_SECS;
// Import stream utilities
use super::stream_utils::flush_reasoning_buffer;

/// Context needed for interaction processing.
/// This is a version of InteractionContext with public fields for the processor.
pub struct ProcessorContext {
    pub chat_id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub pool: DbPool,
    pub rig_service: Arc<RigService>,
    pub storage: Arc<FileStorageService>,
    pub registry: Arc<AgentRegistry>,
    pub session_id: Option<Uuid>,
    pub default_persona: String,
    pub default_context_token_limit: usize,
    pub state: Arc<Mutex<SharedActorState>>,
    pub event_tx: broadcast::Sender<SseEvent>,
    /// Channel to signal tag indexer worker when files are modified
    pub tag_index_tx: mpsc::UnboundedSender<TagIndexMessage>,
}

// ============================================================================
// AGENT CREATION
// ============================================================================

/// Get or create a cached Rig Agent.
pub async fn get_or_create_agent(
    ctx: &ProcessorContext,
    user_id: Uuid,
    session: &crate::models::chat::ChatSession,
    ai_config: &crate::config::AiConfig,
) -> crate::error::Result<Agent> {
    let model = &session.agent_config.model;
    let mode = &session.agent_config.mode;

    // Create new agent (rig::agent::Agent is not Cloneable, so we create fresh each time)
    let agent = ctx.rig_service.create_agent(
        ctx.pool.clone(),
        ctx.storage.clone(),
        ctx.workspace_id,
        ctx.chat_id,
        user_id,
        session,
        ai_config,
        ctx.tag_index_tx.clone(),
    ).await?;

    // Update session metadata with the actual model being used
    if let Some(session_id) = &ctx.session_id {
        tracing::debug!(
            session_id = %session_id,
            model = %model,
            mode = %mode,
            "[Processor] Updating session metadata after agent creation"
        );

        let mut conn = ctx.pool.acquire().await.map_err(crate::error::Error::Sqlx)?;

        // Determine agent type from mode
        let agent_type = match mode.as_str() {
            "plan" => Some(AgentType::Planner),
            "build" => Some(AgentType::Builder),
            _ => Some(AgentType::Assistant),
        };

        // Update session with actual model, mode, and agent type
        if let Err(e) = agent_sessions::update_session_metadata(
            &mut conn,
            *session_id,
            Some(model.clone()),
            Some(mode.clone()),
            agent_type,
            ctx.user_id,
        ).await {
            tracing::warn!(
                session_id = %session_id,
                error = %e,
                "[Processor] Failed to update session metadata after agent creation"
            );
        } else {
            tracing::info!(
                session_id = %session_id,
                model = %model,
                mode = %mode,
                "[Processor] Successfully updated session metadata"
            );
        }
    }

    Ok(agent)
}

// ============================================================================
// STREAM PROCESSING
// ============================================================================

/// Process a generic agent stream, handling cancellation and stream items.
pub async fn process_agent_stream<S, M, E>(
    ctx: &ProcessorContext,
    mut stream: S,
    cancellation_token: &CancellationToken,
    conn: &mut sqlx::PgConnection,
    session: &crate::models::chat::ChatSession,
    item_count: &mut usize,
) -> crate::error::Result<String>
where
    S: futures::Stream<Item = std::result::Result<rig::agent::MultiTurnStreamItem<M>, E>> + Unpin,
    M: std::fmt::Debug + 'static,
    E: std::fmt::Display,
{
    let mut full_response = String::new();
    let mut has_started_responding = false;

    loop {
        // Check for cancellation before each stream iteration
        if cancellation_token.is_cancelled() {
            tracing::info!("[Processor] Cancelled during streaming for chat {}", ctx.chat_id);
            handle_cancellation(ctx, conn, full_response.clone(), "user_cancelled").await?;
            return Err(crate::error::Error::Internal("Chat cancelled by user".to_string()));
        }

        // Use tokio::select! to allow cancellation during stream.next()
        // Also add a timeout to detect stalled API streams
        let stream_timeout = tokio::time::Duration::from_secs(STREAM_READ_TIMEOUT_SECS);
        let item = tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!("[Processor] Cancelled during streaming for chat {}", ctx.chat_id);
                handle_cancellation(ctx, conn, full_response.clone(), "user_cancelled").await?;
                return Err(crate::error::Error::Internal("Chat cancelled by user".to_string()));
            },
            item_result = tokio::time::timeout(stream_timeout, stream.next()) => {
                match item_result {
                    Ok(item) => {
                        match &item {
                            Some(Ok(_)) => tracing::debug!(chat_id = %ctx.chat_id, item_num = *item_count, "Received stream item"),
                            Some(Err(e)) => tracing::error!(chat_id = %ctx.chat_id, error = %e, "Stream error"),
                            None => tracing::info!(chat_id = %ctx.chat_id, "Stream ended (None)"),
                        }
                        item
                    }
                    Err(_) => {
                        // Timeout - API stream stalled
                        tracing::warn!(
                            chat_id = %ctx.chat_id,
                            timeout_secs = STREAM_READ_TIMEOUT_SECS,
                            "[Processor] Stream read timeout - API stalled"
                        );
                        return Err(crate::error::Error::Internal(
                            format!("Stream read timeout after {} seconds - API stalled", STREAM_READ_TIMEOUT_SECS)
                        ));
                    }
                }
            }
        };

        let item = match item {
            Some(i) => i,
            None => {
                tracing::info!(chat_id = %ctx.chat_id, items_received = *item_count, "Stream finished naturally");
                break;
            }
        };

        *item_count += 1;

        match item {
            Err(e) => {
                let error_str = e.to_string();

                // Check for JSON parsing errors in tool calls (common with very long content)
                if error_str.contains("JsonError") || error_str.contains("EOF while parsing") {
                    tracing::error!(
                        chat_id = %ctx.chat_id,
                        error = %error_str,
                        item_num = *item_count,
                        "[Processor] JSON parsing error in tool call - content may be too long or have invalid characters"
                    );
                    return Err(crate::error::Error::Internal(
                        "Tool call JSON parsing failed. The content may be too long or contain invalid characters. \
                         Try using smaller content chunks or check for special characters that need escaping.".to_string()
                    ));
                }

                tracing::error!(
                    chat_id = %ctx.chat_id,
                    error = %e,
                    item_num = *item_count,
                    "Stream item error"
                );
                return Err(crate::error::Error::Internal(format!("Streaming error: {}", e)));
            }
            Ok(stream_item) => {
                if let Err(e) = process_stream_item(
                    ctx,
                    stream_item,
                    &mut full_response,
                    &mut has_started_responding,
                    *item_count,
                    conn,
                    session,
                    cancellation_token,
                ).await {
                    return Err(e);
                }
            }
        }
    }

    Ok(full_response)
}

/// Process a single stream item from either provider.
/// This method is generic over the stream response type to work with both OpenAI and OpenRouter.
async fn process_stream_item<M>(
    ctx: &ProcessorContext,
    stream_item: rig::agent::MultiTurnStreamItem<M>,
    full_response: &mut String,
    has_started_responding: &mut bool,
    item_count: usize,
    conn: &mut sqlx::PgConnection,
    _session: &crate::models::chat::ChatSession,
    _cancellation_token: &CancellationToken,
) -> crate::error::Result<()>
where
    M: std::fmt::Debug + 'static,
{
    match stream_item {
        rig::agent::MultiTurnStreamItem::StreamAssistantItem(content) => {
            match content {
                rig::streaming::StreamedAssistantContent::Text(text) => {
                    // Flush pending reasoning buffer before text chunk
                    if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, conn).await {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            error = %e,
                            "[Processor] Failed to flush reasoning buffer before text"
                        );
                    }

                    tracing::debug!(
                        chat_id = %ctx.chat_id,
                        text_len = text.text.len(),
                        text_preview = %crate::utils::safe_preview(&text.text, 50),
                        "[Processor] [Rig] Received Text chunk"
                    );
                    if !*has_started_responding {
                        tracing::info!("[Processor] [Rig] AI started streaming text response for chat {}", ctx.chat_id);
                        *has_started_responding = true;
                    }
                    full_response.push_str(&text.text);
                    let send_result = ctx.event_tx.send(SseEvent::Chunk { text: text.text.clone() });
                    if let Err(e) = send_result {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            error = %e,
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] Failed to send Chunk event - no receivers or broadcast channel closed"
                        );
                    } else {
                        tracing::debug!(
                            chat_id = %ctx.chat_id,
                            text_len = text.text.len(),
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] Successfully sent Chunk event"
                        );
                    }
                }
                rig::streaming::StreamedAssistantContent::ReasoningDelta { id, reasoning } => {
                    tracing::debug!(
                        chat_id = %ctx.chat_id,
                        reasoning_len = reasoning.len(),
                        id = ?id,
                        "[Processor] [Rig] Received streaming ReasoningDelta chunk"
                    );

                    // Generate reasoning_id on first chunk to link all chunks from this turn
                    {
                        let mut state = ctx.state.lock().await;
                        state.ensure_reasoning_id();
                        // Buffer reasoning chunk (will be saved aggregated later)
                        state.reasoning_buffer.push(reasoning.clone());
                        tracing::debug!(
                            chat_id = %ctx.chat_id,
                            buffer_size = state.reasoning_buffer.len(),
                            "[Processor] Buffered ReasoningDelta chunk"
                        );
                    }

                    // Send streaming reasoning chunk to frontend via Thought event
                    let send_result = ctx.event_tx.send(SseEvent::Thought {
                        agent_id: None,
                        text: reasoning.clone(),
                    });
                    if let Err(e) = send_result {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            event_type = "Thought",
                            error = ?e,
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] FAILED to send event - no receivers"
                        );
                    } else {
                        tracing::trace!(
                            chat_id = %ctx.chat_id,
                            event_type = "Thought",
                            reasoning_len = reasoning.len(),
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] SENT event successfully"
                        );
                    }
                }
                rig::streaming::StreamedAssistantContent::Reasoning(thought) => {
                    tracing::info!(
                        chat_id = %ctx.chat_id,
                        reasoning_parts = thought.reasoning.len(),
                        "[Processor] [Rig] Received final Reasoning (accumulated)"
                    );

                    // Buffer all reasoning parts
                    {
                        let mut state = ctx.state.lock().await;
                        state.ensure_reasoning_id();
                        for part in &thought.reasoning {
                            if !part.trim().is_empty() {
                                state.reasoning_buffer.push(part.clone());
                            }
                        }
                    }

                    // Send to frontend
                    for part in &thought.reasoning {
                        if !part.trim().is_empty() {
                            let send_result = ctx.event_tx.send(SseEvent::Thought {
                                agent_id: None,
                                text: part.clone(),
                            });
                            if let Err(e) = send_result {
                                tracing::error!(
                                    chat_id = %ctx.chat_id,
                                    event_type = "Thought",
                                    error = ?e,
                                    receivers = ctx.event_tx.receiver_count(),
                                    "[Processor] [SSE] FAILED to send event - no receivers"
                                );
                            } else {
                                tracing::trace!(
                                    chat_id = %ctx.chat_id,
                                    event_type = "Thought",
                                    reasoning_len = part.len(),
                                    receivers = ctx.event_tx.receiver_count(),
                                    "[Processor] [SSE] SENT event successfully"
                                );
                            }
                        }
                    }

                    // Flush aggregated reasoning to database
                    if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, conn).await {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            error = %e,
                            "[Processor] Failed to flush reasoning buffer"
                        );
                    }
                }
                rig::streaming::StreamedAssistantContent::ToolCall(tool_call) => {
                    // Flush pending reasoning buffer before tool call
                    if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, conn).await {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            error = %e,
                            "[Processor] Failed to flush reasoning buffer before tool call"
                        );
                    }

                    tracing::info!("[Processor] [Rig] AI calling tool {} for chat {}", tool_call.function.name, ctx.chat_id);

                    // Extract arguments as JSON for persistence
                    let arguments_json = match serde_json::to_value(&tool_call.function.arguments) {
                        Ok(val) => val,
                        Err(e) => {
                            tracing::warn!(
                                chat_id = %ctx.chat_id,
                                tool = %tool_call.function.name,
                                error = %e,
                                "[Processor] Failed to serialize tool arguments for persistence"
                            );
                            serde_json::json!({ "error": "Failed to serialize arguments" })
                        }
                    };

                    // Summarize arguments for persistence to avoid DB bloat
                    let summarized_args = ChatService::summarize_tool_inputs(
                        &tool_call.function.name,
                        &arguments_json,
                    );

                    // Build detailed content for .chat file
                    let args_preview = if let Ok(args_str) = serde_json::to_string_pretty(&summarized_args) {
                        // Truncate if too long for file content
                        if args_str.len() > 500 {
                            format!("{}...\n[Arguments truncated, see metadata for full details]", crate::utils::truncate_safe(&args_str, 500))
                        } else {
                            args_str
                        }
                    } else {
                        "[Could not serialize arguments]".to_string()
                    };
                    let tool_call_content = format!(
                        "AI called tool: {}\nArguments:\n{}",
                        tool_call.function.name,
                        args_preview
                    );

                    // Persist tool call for audit trail
                    let reasoning_id = {
                        let mut state = ctx.state.lock().await;
                        state.ensure_reasoning_id();
                        state.current_reasoning_id.clone()
                    };
                    let metadata = ChatMessageMetadata {
                        message_type: Some("tool_call".to_string()),
                        reasoning_id,
                        tool_name: Some(tool_call.function.name.clone()),
                        tool_arguments: Some(summarized_args),
                        ..Default::default()
                    };
                    if let Err(e) = ChatService::save_stream_event(
                        conn,
                        &ctx.storage,
                        ctx.workspace_id,
                        ctx.chat_id,
                        ChatMessageRole::Tool,
                        tool_call_content,
                        metadata,
                    ).await {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            tool = %tool_call.function.name,
                            error = %e,
                            "[Processor] Failed to persist tool call"
                        );
                    }

                    // Track tool name and arguments for logging when ToolResult arrives
                    {
                        let mut state = ctx.state.lock().await;
                        state.current_tool_name = Some(tool_call.function.name.clone());
                        state.current_tool_args = Some(arguments_json);
                    }

                    let path = tool_call.function.arguments.get("path")
                        .or_else(|| tool_call.function.arguments.get("source"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let tool_name = tool_call.function.name.clone();
                    if let Err(e) = ctx.event_tx.send(SseEvent::Call {
                        tool: tool_call.function.name,
                        path,
                        args: tool_call.function.arguments,
                    }) {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            event_type = "Call",
                            tool = %tool_name,
                            error = ?e,
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] FAILED to send event - no receivers"
                        );
                    } else {
                        tracing::debug!(
                            chat_id = %ctx.chat_id,
                            event_type = "Call",
                            tool = %tool_name,
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] SENT event successfully"
                        );
                    }
                }
                _ => {}
            }
        }
        rig::agent::MultiTurnStreamItem::StreamUserItem(content) => {
            match content {
                rig::streaming::StreamedUserContent::ToolResult(result) => {
                    // Flush reasoning buffer before tool result persistence
                    if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, conn).await {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            error = %e,
                            "[Processor] Failed to flush reasoning buffer before tool result"
                        );
                    }

                    let output = if let Some(rig::completion::message::ToolResultContent::Text(text)) = result.content.iter().next() {
                        text.text.clone()
                    } else {
                        "Tool execution completed".to_string()
                    };

                    // Determine tool success by parsing the response
                    // Tools return ToolResponse {success, result, error} format
                    // Define error detection heuristic once to avoid duplication
                    // Only check for "ToolCallError" to avoid false positives from tool result content
                    let has_error_heuristic = |s: &str| {
                        s.contains("ToolCallError")
                    };

                    let (success, normalized_output) = if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&output) {
                        // Check if this is a ToolResponse format with explicit success field
                        if let Some(success_bool) = result_json.get("success").and_then(|v| v.as_bool()) {
                            // ToolResponse format: {"success": true/false, "result": ..., "error": ...}
                            let normalized = if success_bool {
                                // Success: extract result field
                                result_json.get("result")
                                    .cloned()
                                    .unwrap_or(result_json)
                                    .to_string()
                            } else {
                                // Failure: extract error message
                                result_json.get("error")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| output.clone())
                            };
                            (success_bool, normalized)
                        } else {
                            // No success field - likely a bare result (success)
                            // Check for error patterns in the output
                            let has_error = has_error_heuristic(&output);
                            (!has_error, output.clone())
                        }
                    } else {
                        // Not JSON - use heuristic for plain text
                        let has_error = has_error_heuristic(&output);
                        (!has_error, output.clone())
                    };

                    // Check if this is an ask_user tool result with question_pending
                    // Only ask_user returns question_pending status
                    if success {
                        if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&normalized_output) {
                            // Handle question_pending (ask_user tool)
                            if let Some(status) = result_json.get("status").and_then(|s| s.as_str()) {
                                if status == "question_pending" {
                                    // Extract question data
                                    if let Some(questions) = result_json.get("questions").and_then(|q| q.as_array()) {
                                        let parsed_questions: Vec<crate::models::sse::Question> = questions
                                            .iter()
                                            .filter_map(|q| serde_json::from_value(q.clone()).ok())
                                            .collect();

                                        if let Some(question_id) = result_json.get("question_id").and_then(|id| id.as_str()) {
                                            if let Ok(qid) = uuid::Uuid::parse_str(question_id) {
                                                tracing::info!(
                                                    "[Processor] Emitting QuestionPending event with {} questions for chat {}",
                                                    parsed_questions.len(),
                                                    ctx.chat_id
                                                );
                                                let _ = ctx.event_tx.send(SseEvent::QuestionPending {
                                                    question_id: qid,
                                                    questions: parsed_questions,
                                                    created_at: chrono::Utc::now(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    tracing::info!(
                        "[Processor] [Rig] Tool execution finished for chat {} (success: {}). Output: {}",
                        ctx.chat_id,
                        success,
                        crate::utils::safe_preview(&output, 100)
                    );
                    let send_result = ctx.event_tx.send(SseEvent::Observation { output: normalized_output.clone(), success });
                    if let Err(e) = send_result {
                        tracing::error!(
                            chat_id = %ctx.chat_id,
                            event_type = "Observation",
                            error = ?e,
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] FAILED to send event - no receivers"
                        );
                    } else {
                        tracing::debug!(
                            chat_id = %ctx.chat_id,
                            event_type = "Observation",
                            success = success,
                            receivers = ctx.event_tx.receiver_count(),
                            "[Processor] [SSE] SENT event successfully"
                        );
                    }

                    // Persist tool result for audit trail (BEFORE any state modifications)
                    {
                        let (tool_name_opt, reasoning_id) = {
                            let state = ctx.state.lock().await;
                            (state.current_tool_name.clone(), state.current_reasoning_id.clone())
                        };
                        if let Some(tool_name) = tool_name_opt {
                            // Summarize output for persistence to avoid DB bloat
                            let summarized_output = ChatService::summarize_tool_outputs(&tool_name, &normalized_output);

                            // Build detailed content for .chat file
                            let tool_result_content = format!(
                                "Tool {}: {}\nOutput:\n{}",
                                tool_name,
                                if success { "succeeded" } else { "failed" },
                                summarized_output
                            );

                            let metadata = ChatMessageMetadata {
                                message_type: Some("tool_result".to_string()),
                                reasoning_id,
                                tool_name: Some(tool_name.clone()),
                                tool_output: Some(summarized_output),
                                tool_success: Some(success),
                                ..Default::default()
                            };
                            if let Err(e) = ChatService::save_stream_event(
                                conn,
                                &ctx.storage,
                                ctx.workspace_id,
                                ctx.chat_id,
                                ChatMessageRole::Tool,
                                tool_result_content,
                                metadata,
                            ).await {
                                tracing::error!(
                                    chat_id = %ctx.chat_id,
                                    tool = %tool_name,
                                    error = %e,
                                    "[Processor] Failed to persist tool result"
                                );
                            }
                        } else {
                            tracing::warn!(
                                chat_id = %ctx.chat_id,
                                "[Processor] Tool result without tracked tool name; skipping persistence"
                            );
                        }
                    }

                    // Clear current tool name and arguments
                    {
                        let mut state = ctx.state.lock().await;
                        state.current_tool_name = None;
                        state.current_tool_args = None;
                    }
                }
            }
        }
        rig::agent::MultiTurnStreamItem::FinalResponse(final_response) => {
            // Flush reasoning buffer before final response
            if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, conn).await {
                tracing::error!(
                    chat_id = %ctx.chat_id,
                    error = %e,
                    "[Processor] Failed to flush reasoning buffer before final response"
                );
            }

            tracing::info!(
                chat_id = %ctx.chat_id,
                response_len = final_response.response().len(),
                response_text = %final_response.response(),
                usage = ?final_response.usage(),
                "[Processor] Received FinalResponse from stream"
            );

            // Note: FinalResponse contains the complete text, but we DON'T append it
            // because full_response has already accumulated all Text chunks during streaming.
            // Appending would cause duplication in the saved message.
            //
            // FinalResponse is only used here for logging and usage statistics.
            let response_text = final_response.response();

            // Debug logging to diagnose duplication issues
            tracing::debug!(
                chat_id = %ctx.chat_id,
                full_response_len = full_response.len(),
                final_response_len = response_text.len(),
                "[Processor] FinalResponse received - accumulated={} vs final={}",
                full_response.len(),
                response_text.len()
            );

            if !response_text.is_empty() && !*has_started_responding {
                tracing::info!("[Processor] AI started responding (via FinalResponse) for chat {}", ctx.chat_id);
                *has_started_responding = true;
            }
        }
        // Catch-all for future Rig variants (MultiTurnStreamItem is non-exhaustive)
        _ => {
            tracing::warn!(
                chat_id = %ctx.chat_id,
                item_num = item_count,
                "[Processor] Unhandled stream item variant"
            );
        }
    }

    Ok(())
}

// ============================================================================
// CANCELLATION HANDLING
// ============================================================================

/// Handle user cancellation during streaming.
pub async fn handle_cancellation(
    ctx: &ProcessorContext,
    conn: &mut sqlx::PgConnection,
    partial_response: String,
    reason: &str,
) -> crate::error::Result<()> {
    tracing::info!(
        "[Processor] Handling cancellation for chat {} (reason: {})",
        ctx.chat_id,
        reason
    );

    // Flush any pending reasoning buffer before cancellation persistence
    if let Err(e) = flush_reasoning_buffer(&ctx.state, ctx.chat_id, ctx.workspace_id, &ctx.storage, conn).await {
        tracing::error!(
            chat_id = %ctx.chat_id,
            error = %e,
            "[Processor] Failed to flush reasoning buffer during cancellation"
        );
    }

    // Remove cancellation token - stream is being cancelled
    ctx.registry.remove_cancellation(&ctx.chat_id).await;

    // Get current model for metadata
    let model = ctx.state.lock().await.current_model.clone()
        .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

    // 1. Send Stopped event to all SSE clients
    let _ = ctx.event_tx.send(SseEvent::Stopped {
        reason: reason.to_string(),
        partial_response: if partial_response.is_empty() {
            None
        } else {
            Some(partial_response.clone())
        },
    });

    // 2. Save partial response if there is any text
    if !partial_response.is_empty() {
        tracing::info!(
            "[Processor] Saving partial response ({} chars) for chat {} (model: {})",
            partial_response.len(),
            ctx.chat_id,
            model
        );
        save_partial_response(ctx, conn, partial_response.clone(), model).await?;
    }

    // 3. Add cancellation marker to chat history for AI awareness
    add_cancellation_marker(ctx, conn, reason).await?;

    // 4. Clear the cancellation token for this interaction
    ctx.state.lock().await.current_cancellation_token = None;

    // 5. Clear reasoning ID for next interaction
    ctx.state.lock().await.current_reasoning_id = None;

    // 6. Clear processing flag - actor is no longer actively processing
    ctx.state.lock().await.is_actively_processing = false;

    Ok(())
}

/// Save partial response when interaction is cancelled.
pub async fn save_partial_response(
    ctx: &ProcessorContext,
    conn: &mut sqlx::PgConnection,
    content: String,
    model: String,
) -> crate::error::Result<()> {
    ChatService::save_message(
        conn,
        &ctx.storage,
        ctx.workspace_id,
        NewChatMessage {
            file_id: ctx.chat_id,
            workspace_id: ctx.workspace_id,
            role: ChatMessageRole::Assistant,
            content,
            metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata {
                model: Some(model),
                ..Default::default()
            }),
        },
    )
    .await?;
    Ok(())
}

/// Add a cancellation marker to chat history for AI awareness.
pub async fn add_cancellation_marker(
    ctx: &ProcessorContext,
    conn: &mut sqlx::PgConnection,
    reason: &str,
) -> crate::error::Result<()> {
    let marker_content = format!(
        "[System: Response was interrupted by user ({})]",
        reason
    );

    ChatService::save_message(
        conn,
        &ctx.storage,
        ctx.workspace_id,
        NewChatMessage {
            file_id: ctx.chat_id,
            workspace_id: ctx.workspace_id,
            role: ChatMessageRole::System,
            content: marker_content,
            metadata: sqlx::types::Json(crate::models::chat::ChatMessageMetadata::default()),
        },
    )
    .await?;
    Ok(())
}
