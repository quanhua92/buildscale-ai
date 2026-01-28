use crate::error::Error;
use crate::models::requests::{
    EditArgs, GrepArgs, LsArgs, MkdirArgs, MvArgs, ReadArgs, RmArgs, TouchArgs, WriteArgs,
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
    if let Some(obj) = schema.as_object_mut() {
        obj.insert("additionalProperties".to_string(), serde_json::json!(false));

        // Ensure all properties are in 'required' list for OpenAI strict mode
        if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
            let all_keys: Vec<String> = properties.keys().cloned().collect();
            obj.insert("required".to_string(), serde_json::json!(all_keys));
        }
    }
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
            pub user_id: Uuid,
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
                    ToolDefinition {
                        name,
                        description: $description.to_string(),
                        parameters: enforce_strict_schema(
                            serde_json::to_value(schemars::schema_for!($args_type))
                                .unwrap_or_default(),
                        ),
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
                let user_id = self.user_id;

                async move {
                    let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
                    let tool = $core_tool;
                    let response = tools::Tool::execute(
                        &tool,
                        &mut conn,
                        &storage,
                        workspace_id,
                        user_id,
                        serde_json::to_value(args)?,
                    )
                    .await?;
                    if response.success {
                        Ok(response.result)
                    } else {
                        Err(Error::Internal(
                            response
                                .error
                                .unwrap_or_else(|| "Unknown tool error".to_string()),
                        ))
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
    "Reads the content and hash of a file. For Document types, automatically unwraps the text field. Use this to get the 'hash' before calling 'edit'. PERFORMANCE WARNING: Do NOT use this tool to search for strings in multiple files; use 'grep' instead for efficiency."
);

define_rig_tool!(
    RigWriteTool,
    tools::write::WriteTool,
    WriteArgs,
    "write",
    "Creates a NEW file or completely OVERWRITES an existing file. SAFETY WARNING: This tool is destructive and bypasses concurrency checks. For modifying existing code or config files, you MUST prefer 'edit' to ensure safety and preserve surrounding context. Supported file_type: 'document' (default), 'canvas', 'whiteboard'. DO NOT use 'text' or 'json' as types."
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
