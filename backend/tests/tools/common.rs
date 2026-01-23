//! Tool execution test helpers
//!
//! This module provides helper functions for testing the tools API.
//!
//! These helpers allow tests to execute tools (ls, read, write, rm)
//! via HTTP requests and verify to results.

#![allow(clippy::type_complexity)]

use crate::common::{TestApp, TestAppOptions, register_and_login, create_workspace};

/// Execute tool via HTTP
///
/// Makes a POST request to the /tools endpoint with the given tool name and arguments.
///
/// # Arguments
/// * `app` - TestApp instance
/// * `workspace_id` - Workspace ID string
/// * `token` - Authentication token string
/// * `tool` - Tool name ("ls", "read", "write", "rm")
/// * `args` - Tool arguments as JSON value
///
/// # Returns
/// * `reqwest::Response` - HTTP response from server
///
/// # Example
/// \`\`\`no_run
/// let response = execute_tool(&app, &workspace_id, &token, "read", serde_json::json!({"path": "/file.txt"})).await;
/// assert_eq!(response.status(), 200);
/// \`\`
pub async fn execute_tool(
///     app: &TestApp,
///     workspace_id: &str,
///     token: &str,
///     tool: &str,
///     args: serde_json::Value,
/// ) -> reqwest::Response {
///     app.client
///         .post(&format!("{}/api/v1/workspaces/{}/tools", app.address, workspace_id))
///         .header("Authorization", format!("Bearer {}", token))
///         .json(&serde_json::json!({
///             "tool": tool,
///             "args": args
///         }))
///         .send()
///         .await
/// }

/// Write file via tool
///
/// Creates a file with the given content using the write tool.
///
/// # Arguments
/// * `app` - TestApp instance
/// * `workspace_id` - Workspace ID string
/// * `token` - Authentication token string
/// * `path` - File path
/// * `content` - File content as JSON value
///
/// # Returns
/// * `String` - File ID of created/updated file
///
/// # Example
/// \`\`\`no_run
/// let file_id = write_file(&app, &workspace_id, &token, "/test.txt", serde_json::json!({"text": "hello"})).await;
/// \`\`

/// Read file via tool
///
/// Reads the content of a file using the read tool.
///
/// # Arguments
/// * `app` - TestApp instance
/// * `workspace_id` - Workspace ID string
/// * `token` - Authentication token string
/// * `path` - File path
///
/// # Returns
/// * `serde_json::Value` - File content as JSON value
///
/// # Example
/// \`\`\`no_run
/// let content = read_file(&app, &workspace_id, &token, "/test.txt").await;
/// assert_eq!(content["text"], "hello");
/// \`\`

/// Delete file via tool
///
/// Deletes a file using the rm tool.
///
/// # Arguments
/// * `app` - TestApp instance
/// * `workspace_id` - Workspace ID string
/// * `token` - Authentication token string
/// * `path` - File path
///
/// # Returns
/// * Nothing (unit)
///
/// # Example
/// \`\`\`no_run
/// delete_file(&app, &workspace_id, &token, "/test.txt").await;
/// \`\`
