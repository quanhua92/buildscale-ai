use crate::error::{Error, Result};
use crate::models::chat::{ChatAttachment, ChatMessageMetadata, ChatMessageRole, NewChatMessage, DEFAULT_CHAT_MODEL};
use crate::models::files::{FileType, FileStatus, NewFile};
use crate::models::requests::{CreateChatRequest, PostChatMessageRequest, UpdateChatRequest};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::actor::ChatActor;
use crate::services::chat::registry::AgentCommand;
use crate::state::AppState;
use crate::middleware::auth::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{Extension, Json};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio_stream::wrappers::{BroadcastStream, IntervalStream};
use uuid::Uuid;
use futures::StreamExt;

/// Maximum length of goal text to include in chat file name.
const CHAT_NAME_GOAL_SNIPPET_LENGTH: usize = 80;

/// Helper function to get the persona for a chat based on its mode.
///
/// This extracts the mode from the chat's app_data and determines the appropriate
/// persona (role) for the agent.
async fn get_chat_persona(
    conn: &mut sqlx::PgConnection,
    chat_id: Uuid,
) -> Result<String> {
    // Get the latest version to determine the mode from app_data
    let latest_version = queries::files::get_latest_version(conn, chat_id).await?;

    // Extract mode from app_data, default to "plan" if not set
    let mode = latest_version
        .app_data
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("plan");

    // Determine role from mode
    let role = match mode {
        "build" => Some("builder"),
        "plan" => Some("planner"),
        _ => None,
    };

    Ok(crate::agents::get_persona(role, Some(mode), None))
}

pub async fn create_chat(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
    Json(req): Json<CreateChatRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    tracing::info!("[ChatHandler] Creating chat in workspace {} for user {}", workspace_id, user.id);
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    // 1. Ensure the /chats folder exists and get its ID
    let chats_folder_id = crate::services::files::ensure_path_exists(
        &mut conn,
        workspace_id,
        "chats",
        user.id,
    ).await?;

    // 2. Create the .chat file identity with TEMPORARY path/slug
    // We need to create the file first to get its ID, then update the path
    let chat_file = queries::files::create_file_identity(&mut conn, NewFile {
        workspace_id,
        parent_id: chats_folder_id,
        author_id: user.id,
        file_type: FileType::Chat,
        status: FileStatus::Ready,
        name: {
            let snippet_end = req.goal.char_indices()
                .nth(CHAT_NAME_GOAL_SNIPPET_LENGTH)
                .map_or(req.goal.len(), |(idx, _)| idx);
            format!("Chat: {}", &req.goal[..snippet_end])
        },
        slug: "chat-temp".to_string(),  // Temporary slug
        path: "/chats/chat-temp".to_string(),  // Temporary path
        is_virtual: true,
        is_remote: false,
        permission: 600,
    }).await?;

    // 3. Update the file with its actual ID in the path/slug
    let correct_path = format!("/chats/chat-{}.chat", chat_file.id);
    let correct_slug = format!("chat-{}.chat", chat_file.id);
    let chat_file = queries::files::update_file_path_and_slug(&mut conn, chat_file.id, correct_path, correct_slug).await?;

    tracing::info!("[ChatHandler] Chat file created: {} (ID: {})", chat_file.path, chat_file.id);

    // 4. Create initial version with config in app_data
    // New chats default to Plan Mode (user switches to Build Mode when ready)
    let app_data = serde_json::json!({
        "goal": req.goal,
        "agents": req.agents,
        "model": req.model.clone().unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string()),
        "persona": crate::agents::get_persona(req.role.as_deref(), Some("plan"), None),
        "temperature": 0.7,
        "mode": "plan",
        "plan_file": null
    });

    let version = queries::files::create_version(&mut conn, crate::models::files::NewFileVersion {
        id: None,
        file_id: chat_file.id,
        workspace_id,
        branch: "main".to_string(),
        app_data,
        hash: "initial".to_string(),
        author_id: Some(user.id),
    }).await?;

    queries::files::update_latest_version_id(&mut conn, chat_file.id, version.id).await?;

    // 5. Persist initial goal message via Service (triggers write-through snapshot)
    use crate::services::chat::ChatService;

    // Get model for metadata (from request or default)
    let model_for_metadata = req.model.clone()
        .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

    ChatService::save_message(&mut conn, &state.storage, workspace_id, NewChatMessage {
        file_id: chat_file.id,
        workspace_id,
        role: ChatMessageRole::User,
        content: req.goal,
        metadata: sqlx::types::Json(ChatMessageMetadata {
            attachments: req.files.unwrap_or_default().into_iter().map(|f| ChatAttachment::File {
                file_id: f,
                version_id: None,
            }).collect(),
            model: Some(model_for_metadata),
            ..Default::default()
        }),
    }).await?;

    // 6. Trigger Actor immediately for the initial goal
    let event_tx = state.agents.get_or_create_bus(chat_file.id).await;

    // Determine the mode from the request role (defaults to "plan" if not specified)
    let mode = match req.role.as_deref() {
        Some("builder") => "build",
        Some("planner") | None => "plan",  // Default to plan mode for new chats
        _ => "chat",
    };

    let handle = ChatActor::spawn(crate::services::chat::actor::ChatActorArgs {
        chat_id: chat_file.id,
        workspace_id,
        user_id: user.id,
        pool: state.pool.clone(),
        rig_service: state.rig_service.clone(),
        storage: state.storage.clone(),
        registry: state.agents.clone(),
        default_persona: crate::agents::get_persona(req.role.as_deref(), Some(mode), None),
        default_context_token_limit: state.config.ai.default_context_token_limit,
        event_tx,
        inactivity_timeout: std::time::Duration::from_secs(state.config.ai.actor_inactivity_timeout_seconds),
    });
    state.agents.register(chat_file.id, handle.clone()).await;

    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction {
        user_id: user.id,
    }).await;

    tracing::info!("[ChatHandler] Agent seeded and triggered for new chat {}", chat_file.id);

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "chat_id": chat_file.id,
            "plan_id": null
        })),
    ))
}

pub async fn get_chat_events(
    State(state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    tracing::info!(
        chat_id = %chat_id,
        workspace_id = %workspace_id,
        user_id = %_user.id,
        "[SSE] CONNECTION_REQUEST - Client connecting to chat events"
    );

    // 1. Get or create persistent bus
    let event_tx = state.agents.get_or_create_bus(chat_id).await;

    // Log when sending SessionInit
    tracing::debug!(
        chat_id = %chat_id,
        "[SSE] SENDING SessionInit event"
    );

    // 2. Ensure actor is alive (rehydrate if needed)
    if state.agents.get_handle(&chat_id).await.is_none() {
        tracing::info!("[ChatHandler] Rehydrating ChatActor for chat {}", chat_id);
        let handle = ChatActor::spawn(crate::services::chat::actor::ChatActorArgs {
            chat_id,
            workspace_id,
            user_id: _user.id,
            pool: state.pool.clone(),
            rig_service: state.rig_service.clone(),
            storage: state.storage.clone(),
            registry: state.agents.clone(),
            default_persona: {
                let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;
                get_chat_persona(&mut conn, chat_id).await?
            },
            default_context_token_limit: state.config.ai.default_context_token_limit,
            event_tx: event_tx.clone(),
            inactivity_timeout: std::time::Duration::from_secs(state.config.ai.actor_inactivity_timeout_seconds),
        });
        state.agents.register(chat_id, handle).await;
    };

    // 3. Send initial session_init event
    let init_event = SseEvent::SessionInit {
        chat_id,
        plan_id: None,
    };
    let init_data = serde_json::to_string(&init_event).map_err(Error::Json)?;
    let init_stream = stream::once(async move { Ok(Event::default().data(init_data)) });

    // 4. Stream from persistent broadcast channel
    let current_receiver_count = event_tx.receiver_count();
    tracing::info!(
        chat_id = %chat_id,
        receiver_count = current_receiver_count + 1, // +1 for this new connection
        "[SSE] SUBSCRIBED - Client now receiving events (total receivers: {})",
        current_receiver_count + 1
    );
    let broadcast_stream = BroadcastStream::new(event_tx.subscribe())
        .filter_map(move |msg| async move {
            match msg {
                Ok(event) => {
                    tracing::debug!(
                        chat_id = %chat_id,
                        event_type = ?std::mem::discriminant(&event),
                        "[SSE] Broadcasting event to client"
                    );
                    match serde_json::to_string(&event) {
                        Ok(data) => Some(Ok(Event::default().data(data))),
                        Err(e) => {
                            tracing::error!("[SSE] Failed to serialize SSE event: {:?}", e);
                            None
                        }
                    }
                },
                Err(_) => {
                    tracing::warn!("[SSE] broadcast receiver lag - client disconnected");
                    None
                }
            }
        });

    // 3. Heartbeat stream (Ping every 15 seconds)
    let heartbeat_stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(15)))
        .map(|_| {
            let event = SseEvent::Ping;
            let data = serde_json::to_string(&event).unwrap_or_default();
            Ok(Event::default().data(data))
        });

    // Chain them together and select with heartbeat
    let main_stream = init_stream.chain(broadcast_stream);
    let combined_stream = stream::select(main_stream, heartbeat_stream);

    Ok(Sse::new(combined_stream).keep_alive(KeepAlive::default()))
}

pub async fn post_chat_message(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PostChatMessageRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    tracing::info!("[ChatHandler] Received message for chat {} from user {}", chat_id, user.id);
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    // 1. Append message to DB (Persistence first!) via Service for Write-Through
    use crate::services::chat::ChatService;

    // Get model for metadata (from request or current chat config)
    let model_for_metadata = if let Some(ref model) = req.model {
        model.clone()
    } else {
        // Get from current chat config
        let version = queries::files::get_latest_version(&mut conn, chat_id).await?;
        let agent_config: crate::models::chat::AgentConfig = serde_json::from_value(version.app_data)
            .unwrap_or_else(|_| crate::models::chat::AgentConfig {
                agent_id: None,
                model: DEFAULT_CHAT_MODEL.to_string(),
                temperature: 0.7,
                persona_override: None,
                previous_response_id: None,
                mode: "plan".to_string(),
                plan_file: None,
            });
        agent_config.model
    };

    ChatService::save_message(&mut conn, &state.storage, workspace_id, NewChatMessage {
        file_id: chat_id,
        workspace_id,
        role: ChatMessageRole::User,
        content: req.content.clone(),
        metadata: sqlx::types::Json(ChatMessageMetadata {
            model: Some(model_for_metadata),
            question_answer: req.metadata.as_ref()
                .and_then(|m| m.get("question_answer"))
                .and_then(|qa| serde_json::from_value(qa.clone()).ok()),
            ..Default::default()
        }),
    }).await?;

    // 2. Update model if provided
    if let Some(new_model) = req.model {
        tracing::info!("[ChatHandler] Updating model for chat {} to {}", chat_id, new_model);
        ChatService::update_chat_model(&mut conn, workspace_id, chat_id, new_model).await?;
    }

    // 2.5. Update chat name from new message content
    let content_trimmed = req.content.trim();
    if !content_trimmed.is_empty() {
        const CHAT_NAME_UPDATE_LENGTH: usize = 80;
        let new_chat_name = ChatService::generate_chat_name(content_trimmed, CHAT_NAME_UPDATE_LENGTH);
        if let Err(e) = ChatService::update_chat_name(&mut conn, chat_id, new_chat_name).await {
            tracing::warn!("[ChatHandler] Failed to update chat name: {}", e);
            // Don't fail the request if name update fails
        }
    }

    // 3. Signal Actor
    let handle = if let Some(handle) = state.agents.get_handle(&chat_id).await {
        handle
    } else {
        // Rehydrate actor
        let event_tx = state.agents.get_or_create_bus(chat_id).await;
        let handle = ChatActor::spawn(crate::services::chat::actor::ChatActorArgs {
            chat_id,
            workspace_id,
            user_id: user.id,
            pool: state.pool.clone(),
            rig_service: state.rig_service.clone(),
            storage: state.storage.clone(),
            registry: state.agents.clone(),
            default_persona: {
                let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;
                get_chat_persona(&mut conn, chat_id).await?
            },
            default_context_token_limit: state.config.ai.default_context_token_limit,
            event_tx,
            inactivity_timeout: std::time::Duration::from_secs(state.config.ai.actor_inactivity_timeout_seconds),
        });
        state.agents.register(chat_id, handle.clone()).await;
        handle
    };

    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction {
        user_id: user.id,
    }).await;

    tracing::info!("[ChatHandler] Interaction command sent to actor for chat {}", chat_id);

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "status": "accepted" })),
    ))
}

pub async fn get_chat(
    State(state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::models::chat::ChatSession>> {
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    let session = crate::services::chat::ChatService::get_chat_session(
        &mut conn,
        workspace_id,
        chat_id,
    ).await?;

    Ok(Json(session))
}

pub async fn update_chat(
    State(state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateChatRequest>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        "[ChatHandler] Updating chat {} in workspace {} with app_data: {:?}",
        chat_id,
        workspace_id,
        req.app_data
    );
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    // Extract mode and plan_file from request
    let mode = req.app_data
        .get("mode")
        .and_then(|m| m.as_str())
        .unwrap_or("plan")
        .to_string();

    let plan_file = req.app_data
        .get("plan_file")
        .and_then(|p| {
            if p.is_null() {
                None
            } else {
                p.as_str()
            }
        })
        .map(|s| s.to_string());

    // Validate mode
    if mode != "plan" && mode != "build" {
        return Err(Error::Validation(crate::error::ValidationErrors::Single {
            field: "mode".to_string(),
            message: "mode must be either 'plan' or 'build'".to_string(),
        }));
    }

    // Get current version to check for mode transition
    let old_mode = if let Ok(version) = crate::queries::files::get_latest_version(&mut conn, chat_id).await {
        let agent_config: crate::models::chat::AgentConfig = serde_json::from_value(version.app_data)
            .unwrap_or_else(|_| crate::models::chat::AgentConfig {
                agent_id: None,
                model: DEFAULT_CHAT_MODEL.to_string(),
                temperature: 0.7,
                persona_override: None,
                previous_response_id: None,
                mode: "plan".to_string(),
                plan_file: None,
            });
        Some(agent_config.mode)
    } else {
        None
    };

    // Update chat metadata
    use crate::services::chat::ChatService;
    ChatService::update_chat_metadata(
        &mut conn,
        &state.storage,
        workspace_id,
        chat_id,
        mode.clone(),
        plan_file.clone(),
    ).await?;

    // Emit SSE event if mode changed
    if old_mode.as_deref() != Some(mode.as_str()) {
        let event_tx = state.agents.get_or_create_bus(chat_id).await;
        let _ = event_tx.send(SseEvent::ModeChanged {
            mode: mode.clone(),
            plan_file: plan_file.clone(),
        });

        tracing::info!(
            "[ChatHandler] Mode changed from {:?} to {} for chat {}",
            old_mode,
            mode,
            chat_id
        );

        // Clear agent cache to force new agent creation with updated mode
        state.agents.remove(&chat_id).await;
    }

    Ok(Json(serde_json::json!({
        "mode": mode,
        "plan_file": plan_file
    })))
}

pub async fn stop_chat_generation(
    State(state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        "[ChatHandler] Stop requested for chat {} in workspace {}",
        chat_id,
        workspace_id
    );

    // First, try to cancel via the active_cancellations HashMap
    // This works even if the actor has exited
    let cancelled_via_hashmap = state.agents.cancel_stream(&chat_id).await;

    if cancelled_via_hashmap {
        tracing::info!(
            "[ChatHandler] Successfully cancelled stream via HashMap for chat {}",
            chat_id
        );
        return Ok(Json(serde_json::json!({
            "status": "cancelled",
            "chat_id": chat_id
        })));
    }

    // If not in HashMap, try to cancel via actor command
    tracing::debug!(
        "[ChatHandler] No active stream in HashMap for chat {}, trying actor command",
        chat_id
    );

    // Get the actor handle
    let handle = state
        .agents
        .get_handle(&chat_id)
        .await
        .ok_or_else(|| Error::NotFound(format!("Chat actor not found and no active stream for chat {}", chat_id)))?;

    // Create a one-shot channel for response
    let (responder, response) = oneshot::channel();
    let responder = Arc::new(Mutex::new(Some(responder)));

    // Send cancel command
    handle
        .command_tx
        .send(AgentCommand::Cancel {
            reason: "user_cancelled".to_string(),
            responder,
        })
        .await
        .map_err(|_| Error::Internal("Failed to send cancel command".into()))?;

    // Wait for acknowledgment
    let _result = response
        .await
        .map_err(|_| Error::Internal("Cancel acknowledgment failed".into()))??;

    tracing::info!("[ChatHandler] Cancel successful for chat {}", chat_id);

    Ok(Json(serde_json::json!({
        "status": "cancelled",
        "chat_id": chat_id
    })))
}

/// GET /workspaces/{id}/chats/{chat_id}/context
///
/// Returns detailed information about everything sent to the AI for debugging
/// and context visualization purposes.
pub async fn get_chat_context(
    State(state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<crate::models::chat::ChatContextResponse>> {
    tracing::info!(
        "[ChatHandler] Getting context for chat {} in workspace {}",
        chat_id, workspace_id
    );

    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    let context_response = crate::services::chat::ChatService::get_context_info(
        &mut conn,
        &state.storage,
        workspace_id,
        chat_id,
        &crate::agents::get_persona(None, None, None),
        state.config.ai.default_context_token_limit,
    ).await?;

    Ok(Json(context_response))
}
