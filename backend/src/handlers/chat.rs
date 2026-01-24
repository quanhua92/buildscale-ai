use crate::error::{Error, Result};
use crate::models::chat::{ChatMessageMetadata, ChatMessageRole, NewChatMessage, ChatAttachment};
use crate::models::files::{FileType, FileStatus, NewFile};
use crate::models::requests::{CreateChatRequest, PostChatMessageRequest};
use crate::models::sse::SseEvent;
use crate::queries;
use crate::services::chat::actor::ChatActor;
use crate::services::chat::registry::AgentCommand;
use crate::services::chat::rig_engine::RigService;
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
use tokio_stream::wrappers::{BroadcastStream, IntervalStream};
use uuid::Uuid;
use futures::StreamExt;
use secrecy::ExposeSecret;

pub async fn create_chat(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
    Json(req): Json<CreateChatRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    // 1. Create the .chat file identity
    let chat_file = queries::files::create_file_identity(&mut conn, NewFile {
        workspace_id,
        parent_id: None,
        author_id: user.id,
        file_type: FileType::Chat,
        status: FileStatus::Ready,
        name: format!("Chat: {}", &req.goal[..std::cmp::min(req.goal.len(), 20)]),
        slug: format!("chat-{}", Uuid::now_v7()),
        path: format!("/chats/chat-{}", Uuid::now_v7()),
        is_virtual: true,
        is_remote: false,
        permission: 600,
    }).await?;

    // 2. Create initial version with config in app_data
    let app_data = serde_json::json!({
        "goal": req.goal,
        "agents": req.agents,
        "model": "gpt-4o",
        "temperature": 0.7
    });

    let version = queries::files::create_version(&mut conn, crate::models::files::NewFileVersion {
        file_id: chat_file.id,
        workspace_id,
        branch: "main".to_string(),
        content_raw: serde_json::json!({"messages": []}),
        app_data,
        hash: "initial".to_string(),
        author_id: Some(user.id),
    }).await?;

    queries::files::update_latest_version_id(&mut conn, chat_file.id, version.id).await?;

    // 3. Persist initial goal message
    queries::chat::insert_chat_message(&mut conn, NewChatMessage {
        file_id: chat_file.id,
        workspace_id,
        role: ChatMessageRole::User,
        content: req.goal,
        metadata: serde_json::to_value(ChatMessageMetadata {
            attachments: req.files.unwrap_or_default().into_iter().map(|f| ChatAttachment::File {
                file_id: f,
                version_id: None,
            }).collect(),
            ..Default::default()
        })?,
    }).await?;

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
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let handle = if let Some(handle) = state.agents.get_handle(&chat_id).await {
        handle
    } else {
        // Rehydrate/Spawn actor
        let config = crate::load_config().expect("Failed to load config");
        let rig_service = Arc::new(RigService::new(config.ai.openai_api_key.expose_secret()));
        let handle = ChatActor::spawn(chat_id, workspace_id, state.pool.clone(), rig_service);
        state.agents.register(chat_id, handle.clone()).await;
        handle
    };

    // 1. Send initial session_init event
    let init_event = SseEvent::SessionInit {
        chat_id,
        plan_id: None,
    };
    let init_data = serde_json::to_string(&init_event).unwrap_or_default();
    let init_stream = stream::once(async move { Ok(Event::default().data(init_data)) });

    // 2. Stream from broadcast channel
    let broadcast_stream = BroadcastStream::new(handle.event_tx.subscribe())
        .filter_map(|msg| async move {
            match msg {
                Ok(event) => {
                    let data = serde_json::to_string(&event).ok()?;
                    Some(Ok(Event::default().data(data)))
                }
                Err(_) => None,
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

    Sse::new(combined_stream).keep_alive(KeepAlive::default())
}

pub async fn post_chat_message(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((workspace_id, chat_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PostChatMessageRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let mut conn = state.pool.acquire().await.map_err(Error::Sqlx)?;

    // 1. Append message to DB (Persistence first!)
    queries::chat::insert_chat_message(&mut conn, NewChatMessage {
        file_id: chat_id,
        workspace_id,
        role: ChatMessageRole::User,
        content: req.content.clone(),
        metadata: serde_json::Value::Object(serde_json::Map::new()),
    }).await?;

    // 2. Touch file
    queries::files::touch_file(&mut conn, chat_id).await?;

    // 3. Signal Actor
    let handle = if let Some(handle) = state.agents.get_handle(&chat_id).await {
        handle
    } else {
        // Rehydrate actor
        let config = crate::load_config().expect("Failed to load config");
        let rig_service = Arc::new(RigService::new(config.ai.openai_api_key.expose_secret()));
        let handle = ChatActor::spawn(chat_id, workspace_id, state.pool.clone(), rig_service);
        state.agents.register(chat_id, handle.clone()).await;
        handle
    };

    let _ = handle.command_tx.send(AgentCommand::ProcessInteraction {
        user_id: user.id,
        content: req.content,
    }).await;

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "status": "accepted" })),
    ))
}
