use crate::error::Error;
use crate::models::requests::{
    EditArgs, GrepArgs, GlobArgs, LsArgs, FileInfoArgs, MkdirArgs, MvArgs, ReadArgs, ReadMultipleFilesArgs, RmArgs, TouchArgs, WriteArgs,
    FindArgs, CatArgs,
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

/// Macro to generate Rig-compatible wrapper for BuildScale tools.
///
/// This macro reduces boilerplate by generating the struct definition and RigTool impl
/// for each workspace tool. All tools follow the same pattern with only minor variations
/// in tool name, args type, and core tool type.
///
/// # Arguments
/// * `$rig_tool_name` - Name of the generated struct (e.g., RigLsTool)
/// * `$core_tool:path` - Path to the core tool type (e.g., tools::ls::LsTool)
/// * `$args_type:ty` - Type of the args (e.g., LsArgs)
/// * `$name:expr` - Tool name as string literal (e.g., "ls")
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
///     "ls"
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
        $name:expr
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
            type Args = Option<$args_type>;
            type Output = serde_json::Value;

            const NAME: &'static str = $name;

            fn definition(
                &self,
                _prompt: String,
            ) -> impl Future<Output = ToolDefinition> + Send + Sync {
                let name = Self::NAME.to_string();
                async move {
                    // Use the core tool's hardcoded definition and description
                    use crate::tools::Tool;
                    let core_tool = $core_tool;
                    let schema = core_tool.definition();

                    ToolDefinition {
                        name,
                        description: core_tool.description().to_string(),
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
                    // Validate that arguments were provided
                    let args = args.ok_or_else(|| {
                        Error::Validation(crate::error::ValidationErrors::Single {
                            field: "arguments".to_string(),
                            message: format!(
                                "Tool '{}' requires arguments. You must provide all required fields as a JSON object. \
                                For example, {{\"pattern\": \"your_search_term\"}}. \
                                Refer to the tool's JSON schema definition for the required fields.",
                                $name
                            ),
                        })
                    })?;
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
    "ls"
);

define_rig_tool!(
    RigReadTool,
    tools::read::ReadTool,
    ReadArgs,
    "read"
);

define_rig_tool!(
    RigWriteTool,
    tools::write::WriteTool,
    WriteArgs,
    "write"
);

define_rig_tool!(
    RigRmTool,
    tools::rm::RmTool,
    RmArgs,
    "rm"
);

define_rig_tool!(
    RigMvTool,
    tools::mv::MvTool,
    MvArgs,
    "mv"
);

define_rig_tool!(
    RigTouchTool,
    tools::touch::TouchTool,
    TouchArgs,
    "touch"
);

define_rig_tool!(
    RigEditTool,
    tools::edit::EditTool,
    EditArgs,
    "edit"
);

define_rig_tool!(
    RigGrepTool,
    tools::grep::GrepTool,
    GrepArgs,
    "grep"
);

define_rig_tool!(
    RigMkdirTool,
    tools::mkdir::MkdirTool,
    MkdirArgs,
    "mkdir"
);

// System tools for Plan Mode workflow
define_rig_tool!(
    RigAskUserTool,
    tools::ask_user::AskUserTool,
    AskUserArgs,
    "ask_user"
);

define_rig_tool!(
    RigExitPlanModeTool,
    tools::exit_plan_mode::ExitPlanModeTool,
    ExitPlanModeArgs,
    "exit_plan_mode"
);

// Phase 1: glob, file_info
define_rig_tool!(
    RigGlobTool,
    tools::glob::GlobTool,
    GlobArgs,
    "glob"
);

define_rig_tool!(
    RigFileInfoTool,
    tools::file_info::FileInfoTool,
    FileInfoArgs,
    "file_info"
);

define_rig_tool!(
    RigReadMultipleFilesTool,
    tools::read_multiple_files::ReadMultipleFilesTool,
    ReadMultipleFilesArgs,
    "read_multiple_files"
);

define_rig_tool!(
    RigFindTool,
    tools::find::FindTool,
    FindArgs,
    "find"
);

define_rig_tool!(
    RigCatTool,
    tools::cat::CatTool,
    CatArgs,
    "cat"
);


