use crate::error::Error;
use crate::models::requests::{
    EditArgs, GrepArgs, LsArgs, MkdirArgs, MvArgs, ReadArgs, RmArgs, TouchArgs, WriteArgs,
    AskUserArgs, ExitPlanModeArgs,
};
use crate::services::storage::FileStorageService;
use crate::tools;

use crate::DbPool;
use rig::completion::ToolDefinition;
use rig::tool::Tool as RigTool;
use std::future::Future;
use std::sync::Arc;
use uuid::Uuid;

fn enforce_strict_schema(mut schema: serde_json::Value) -> serde_json::Value {
    tracing::debug!(
        schema_before = %serde_json::to_string(&schema).unwrap_or_default(),
        "enforce_strict_schema called"
    );

    // Recursively add additionalProperties: false to all object schemas
    fn add_additional_properties_recursive(value: &mut serde_json::Value) {
        if let Some(obj) = value.as_object_mut() {
            // Don't modify schemas that use anyOf/oneOf/allOf/not
            // These already define their own validation logic
            if obj.contains_key("anyOf") || obj.contains_key("oneOf") || obj.contains_key("allOf") || obj.contains_key("not") {
                // Still recurse into the anyOf/oneOf arrays to fix nested schemas
                for key in &["anyOf", "oneOf", "allOf"] {
                    if let Some(arr) = obj.get_mut(*key).and_then(|v| v.as_array_mut()) {
                        for item in arr.iter_mut() {
                            add_additional_properties_recursive(item);
                        }
                    }
                }
                if let Some(s) = obj.get_mut("not") {
                    add_additional_properties_recursive(s);
                }
                return;
            }

            // Add additionalProperties: false if type is "object"
            if obj.get("type").and_then(|t| t.as_str()) == Some("object") {
                obj.entry("additionalProperties").or_insert(serde_json::json!(false));
            }

            // Recurse into nested structures
            if let Some(definitions) = obj.get_mut("definitions").and_then(|d| d.as_object_mut()) {
                for (_key, def) in definitions.iter_mut() {
                    add_additional_properties_recursive(def);
                }
            }

            if let Some(properties) = obj.get_mut("properties").and_then(|p| p.as_object_mut()) {
                for (_key, prop) in properties.iter_mut() {
                    add_additional_properties_recursive(prop);
                }
            }

            if let Some(items) = obj.get_mut("items") {
                add_additional_properties_recursive(items);
            }
        }
    }

    // Recursively ensure all object schemas have required arrays
    fn fix_required_arrays(value: &mut serde_json::Value) {
        if let Some(obj) = value.as_object_mut() {
            // Don't modify schemas that use anyOf/oneOf - they have their own validation logic
            if obj.contains_key("anyOf") || obj.contains_key("oneOf") || obj.contains_key("allOf") || obj.contains_key("not") {
                // Still recurse into the arrays to fix nested schemas
                for key in &["anyOf", "oneOf", "allOf"] {
                    if let Some(arr) = obj.get_mut(*key).and_then(|v| v.as_array_mut()) {
                        for item in arr.iter_mut() {
                            fix_required_arrays(item);
                        }
                    }
                }
                if let Some(s) = obj.get_mut("not") {
                    fix_required_arrays(s);
                }
                return;
            }

            // Add all properties to required array for OpenAI strict mode
            if obj.get("type").and_then(|t| t.as_str()) == Some("object") {
                if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
                    let all_keys: Vec<String> = properties.keys().cloned().collect();
                    obj.insert("required".to_string(), serde_json::json!(all_keys));
                }
            }

            // Recurse into nested structures
            if let Some(definitions) = obj.get_mut("definitions").and_then(|d| d.as_object_mut()) {
                for (_key, def) in definitions.iter_mut() {
                    fix_required_arrays(def);
                }
            }

            if let Some(properties) = obj.get_mut("properties").and_then(|p| p.as_object_mut()) {
                for (_key, prop) in properties.iter_mut() {
                    fix_required_arrays(prop);
                }
            }

            if let Some(items) = obj.get_mut("items") {
                fix_required_arrays(items);
            }
        }
    }

    add_additional_properties_recursive(&mut schema);
    fix_required_arrays(&mut schema);

    tracing::debug!(
        schema_after = %serde_json::to_string(&schema).unwrap_or_default(),
        "enforce_strict_schema completed"
    );

    schema
}

/// Macro to generate Rig-compatible wrapper for BuildScale tools.
///
/// This macro reduces boilerplate by generating the struct definition and RigTool impl
/// for each workspace tool. All tools follow the same pattern with only minor variations
/// in tool name, args type, core tool type, and description.
///
/// # Arguments
/// * `$rig_tool_name` - Name of the generated struct (e.g., RigLsTool)
/// * `$core_tool:path` - Path to the core tool type (e.g., tools::ls::LsTool)
/// * `$args_type:ty` - Type of the args (e.g., LsArgs)
/// * `$name:expr` - Tool name as string literal (e.g., "ls")
/// * `$description:expr` - Tool description
///
/// # Example
/// This example demonstrates the macro usage pattern. The macro invocation below
/// generates a complete Rig-compatible tool wrapper with struct and RigTool implementation:
///
/// ```text
/// define_rig_tool!(
///     RigLsTool,
///     tools::ls::LsTool,
///     LsArgs,
///     "ls",
///     "Lists files and folders in a directory within the workspace."
/// );
/// ```
///
/// This expands to:
/// - A struct `RigLsTool` with `pool`, `workspace_id`, and `user_id` fields
/// - A `RigTool` implementation with `definition()` and `call()` methods
/// - Automatic error handling and tool execution logic
macro_rules! define_rig_tool {
    (
        $rig_tool_name:ident,
        $core_tool:path,
        $args_type:ty,
        $name:expr,
        $description:expr
    ) => {
        pub struct $rig_tool_name {
            pub pool: DbPool,
            pub storage: Arc<FileStorageService>,
            pub workspace_id: Uuid,
            pub chat_id: Uuid,
            pub user_id: Uuid,
            pub tool_config: tools::ToolConfig,
        }

        impl RigTool for $rig_tool_name {
            type Error = Error;
            type Args = $args_type;
            type Output = serde_json::Value;

            const NAME: &'static str = $name;

            fn definition(
                &self,
                _prompt: String,
            ) -> impl Future<Output = ToolDefinition> + Send + Sync {
                let name = Self::NAME.to_string();
                async move {
                    let schema_raw = schemars::schema_for!($args_type);
                    tracing::debug!(
                        tool = $name,
                        args_type = std::any::type_name::<$args_type>(),
                        schema_raw = %serde_json::to_string(&schema_raw).unwrap_or_default(),
                        "Generating tool definition"
                    );

                    let schema = enforce_strict_schema(
                        serde_json::to_value(schema_raw).unwrap_or_default(),
                    );

                    tracing::info!(
                        tool = $name,
                        parameters = %serde_json::to_string(&schema).unwrap_or_default(),
                        "Tool definition generated"
                    );

                    ToolDefinition {
                        name,
                        description: $description.to_string(),
                        parameters: schema,
                    }
                }
            }

            fn call(
                &self,
                args: Self::Args,
            ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send {
                let pool = self.pool.clone();
                let storage = self.storage.clone();
                let workspace_id = self.workspace_id;
                let chat_id = self.chat_id;
                let user_id = self.user_id;
                let initial_tool_config = self.tool_config.clone();

                async move {
                    let args_val = serde_json::to_value(args).map_err(Error::Json)?;
                    let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
                    let tool = $core_tool;

                    // Read current mode from database to get fresh ToolConfig
                    // This ensures mode changes mid-stream are respected
                    let tool_config = if let Ok(version) = crate::queries::files::get_latest_version(&mut conn, chat_id).await {
                        let agent_config: crate::models::chat::AgentConfig =
                            serde_json::from_value(version.app_data).unwrap_or_else(|_| {
                                tracing::warn!(
                                    tool = $name,
                                    chat_id = %chat_id,
                                    "Failed to parse agent_config, using defaults"
                                );
                                crate::models::chat::AgentConfig {
                                    agent_id: None,
                                    model: crate::models::chat::DEFAULT_CHAT_MODEL.to_string(),
                                    temperature: 0.7,
                                    persona_override: None,
                                    previous_response_id: None,
                                    mode: "plan".to_string(),
                                    plan_file: None,
                                }
                            });

                        tracing::debug!(
                            tool = $name,
                            chat_id = %chat_id,
                            mode = %agent_config.mode,
                            plan_file = ?agent_config.plan_file,
                            "Read current mode from database for ToolConfig"
                        );

                        crate::tools::ToolConfig {
                            plan_mode: agent_config.mode == "plan",
                            active_plan_path: agent_config.plan_file,
                        }
                    } else {
                        tracing::warn!(
                            tool = $name,
                            chat_id = %chat_id,
                            "Failed to read latest version, using initial ToolConfig"
                        );
                        initial_tool_config
                    };

                    tracing::debug!(
                        tool = $name,
                        workspace_id = %workspace_id,
                        user_id = %user_id,
                        plan_mode = tool_config.plan_mode,
                        args = %args_val,
                        "Executing tool"
                    );

                    let response = tools::Tool::execute(
                        &tool,
                        &mut conn,
                        &storage,
                        workspace_id,
                        user_id,
                        tool_config,
                        args_val.clone(),
                    )
                    .await?;

                    if response.success {
                        tracing::debug!(
                            tool = $name,
                            "Tool execution successful"
                        );
                        Ok(response.result)
                    } else {
                        let error_msg = response
                            .error
                            .unwrap_or_else(|| "Unknown tool error".to_string());

                        tracing::error!(
                            tool = $name,
                            args = %args_val,
                            error = %error_msg,
                            "Tool execution failed"
                        );

                        Err(Error::Internal(format!(
                            "Tool '{}' failed with input {}: {}",
                            $name, args_val, error_msg
                        )))
                    }
                }
            }
        }
    };
}

// Generate all Rig tool wrappers using the macro
define_rig_tool!(
    RigLsTool,
    tools::ls::LsTool,
    LsArgs,
    "ls",
    "Lists files and folders in a directory within the workspace."
);

define_rig_tool!(
    RigReadTool,
    tools::read::ReadTool,
    ReadArgs,
    "read",
    "Reads the content and hash of a file. Content is returned as stored - raw text for text files, JSON for structured data. Use this to get the 'hash' before calling 'edit'. PERFORMANCE WARNING: Do NOT use this tool to search for strings in multiple files; use 'grep' instead for efficiency."
);

define_rig_tool!(
    RigWriteTool,
    tools::write::WriteTool,
    WriteArgs,
    "write",
    "Creates a NEW file or completely OVERWRITES an existing file. SAFETY WARNING: This tool is destructive and bypasses concurrency checks. For modifying existing code or config files, you MUST prefer 'edit' to ensure safety and preserve surrounding context. Content is stored as-is: strings are stored as raw text, JSON objects as structured data. Supported file_type: 'document' (default), 'plan', 'canvas', 'whiteboard'. DO NOT use 'text' or 'json' as types."
);

define_rig_tool!(
    RigRmTool,
    tools::rm::RmTool,
    RmArgs,
    "rm",
    "Deletes a file or empty folder at the specified path."
);

define_rig_tool!(
    RigMvTool,
    tools::mv::MvTool,
    MvArgs,
    "mv",
    "Moves or renames a file. To rename, provide the full new path. To move, provide the new parent directory path."
);

define_rig_tool!(
    RigTouchTool,
    tools::touch::TouchTool,
    TouchArgs,
    "touch",
    "Updates the access and modification times of a file, or creates an empty 'document' file if it doesn't exist. To create directories, use 'mkdir' instead."
);

define_rig_tool!(
    RigEditTool,
    tools::edit::EditTool,
    EditArgs,
    "edit",
    "Edits a file by replacing a UNIQUE search string with a replacement string. Use this for precision changes. You SHOULD provide 'last_read_hash' for safety. Fails if the string is not unique."
);

define_rig_tool!(
    RigGrepTool,
    tools::grep::GrepTool,
    GrepArgs,
    "grep",
    "REQUIRED for searching. Performs a high-performance, workspace-wide regex search across all files. Returns line numbers and matching text. Always use this instead of reading files manually when looking for patterns."
);

define_rig_tool!(
    RigMkdirTool,
    tools::mkdir::MkdirTool,
    MkdirArgs,
    "mkdir",
    "Recursively creates folders to ensure the specified path exists."
);

// System tools for Plan Mode workflow
define_rig_tool!(
    RigAskUserTool,
    tools::ask_user::AskUserTool,
    AskUserArgs,
    "ask_user",
    "Suspends generation to request structured input or confirmation from the user. Supports asking multiple questions in batch. Questions are ephemeral - they exist only in the SSE stream and frontend memory. User answers come through normal chat messages with metadata."
);

define_rig_tool!(
    RigExitPlanModeTool,
    tools::exit_plan_mode::ExitPlanModeTool,
    ExitPlanModeArgs,
    "exit_plan_mode",
    "Transitions the workspace from Plan Mode to Build Mode. Call this after the user approves the implementation plan. Updates chat metadata and prepares the system for executing the approved plan."
);

