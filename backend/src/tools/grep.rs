use crate::{DbConn, error::Result};
use crate::models::requests::{ToolResponse, GrepArgs, GrepResult};
use crate::queries::files as file_queries;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::Tool;

/// Grep tool for searching file contents using regex
///
/// Searches for a regex pattern across all document files in a workspace.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Searches for a regex pattern (Postgres syntax) across all document files. Pattern is required. Optional path_pattern filters results (supports * wildcards, auto-converts to SQL LIKE). Set case_sensitive to false (default) for case-insensitive search. Returns matching file paths with context. Use for discovering code patterns or finding specific content."
    }

    fn definition(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GrepArgs)).unwrap_or(Value::Null)
    }
    
    async fn execute(
        &self,
        conn: &mut DbConn,
        workspace_id: Uuid,
        _user_id: Uuid,
        args: Value,
    ) -> Result<ToolResponse> {
        let grep_args: GrepArgs = serde_json::from_value(args)?;

        // Validate regex pattern to prevent PostgreSQL errors
        // PostgreSQL regex is similar to Rust regex, so we can validate here
        if let Err(e) = regex::Regex::new(&grep_args.pattern) {
            return Ok(ToolResponse {
                success: false,
                result: serde_json::Value::Null,
                error: Some(format!("Invalid regex pattern: {}", e)),
            });
        }

        let path_pattern = grep_args.path_pattern.map(|mut p| {
            // Convert glob-like * to SQL LIKE %
            p = p.replace('*', "%");
            
            // If no wildcards, assume fuzzy matching for discovery
            if !p.contains('%') {
                if p.ends_with('/') {
                    p.push('%');
                } else if !p.contains('.') {
                    // Likely a directory name like "src" -> match children
                    p.push_str("/%");
                } else {
                    // Likely a filename like "main.rs"
                    // If it doesn't start with /, allow it to match anywhere in the path
                    if !p.starts_with('/') {
                        p.insert(0, '%');
                    }
                }
            }
            
            // Ensure leading slash if it looks like an absolute path and doesn't have a wildcard there
            if !p.starts_with('/') && !p.starts_with('%') {
                p.insert(0, '/');
            }
            p
        });

        let matches = file_queries::grep_files(
            conn, 
            workspace_id, 
            &grep_args.pattern, 
            path_pattern.as_deref(), 
            grep_args.case_sensitive.unwrap_or(false)
        ).await?;
        
        let result = GrepResult {
            matches,
        };
        
        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
