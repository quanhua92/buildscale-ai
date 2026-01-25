use crate::error::Error;
use crate::models::requests::{EditArgs, LsArgs, MvArgs, ReadArgs, RmArgs, TouchArgs, WriteArgs};
use crate::tools;
use crate::DbPool;
use rig::completion::ToolDefinition;
use rig::tool::Tool as RigTool;
use std::future::Future;
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
                let workspace_id = self.workspace_id;
                let user_id = self.user_id;

                async move {
                    let mut conn = pool.acquire().await.map_err(Error::Sqlx)?;
                    let tool = $core_tool;
                    let response = tools::Tool::execute(
                        &tool,
                        &mut conn,
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
    "Reads the content of a file. For Document types, automatically unwraps the text field for convenience. For other types (canvas, whiteboard, etc.), returns the raw JSON structure."
);

define_rig_tool!(
    RigWriteTool,
    tools::write::WriteTool,
    WriteArgs,
    "write",
    "Creates or updates a file. For Document types, accepts raw strings (auto-wrapped to {text: ...}) or {text: string} objects. For other types (canvas, whiteboard, etc.), requires the appropriate JSON structure."
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
    "Updates the access and modification times of a file, or creates an empty file if it doesn't exist."
);

define_rig_tool!(
    RigEditTool,
    tools::edit::EditTool,
    EditArgs,
    "edit",
    "Edits a file by replacing a unique search string with a replacement string. Fails if the search string is not found or found multiple times."
);

define_rig_tool!(
    RigEditManyTool,
    tools::edit::EditManyTool,
    EditArgs,
    "edit-many",
    "Edits a file by replacing all occurrences of a search string with a replacement string. Fails if the search string is not found."
);
