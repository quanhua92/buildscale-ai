use crate::{DbConn, error::{Result, Error}, models::requests::{ToolResponse, AskUserArgs, AskUserResult}};
use crate::services::storage::FileStorageService;
use uuid::Uuid;
use serde_json::Value;
use async_trait::async_trait;
use super::{Tool, ToolConfig};

/// Ask user tool for human-in-the-loop interactions
///
/// This tool allows the AI to request structured input from users during execution.
/// Questions are ephemeral - they exist only in SSE stream and frontend memory.
pub struct AskUserTool;

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &'static str {
        "ask_user"
    }

    fn description(&self) -> &'static str {
        r#"Requests user input. Returns question_pending status.

SCHEMAS:
- String enum: Radio buttons (add "buttons" field)
- Array type: Checkboxes (NO buttons field)

EXAMPLES:
Radio: {"name":"c","question":"Pick:","schema":"{\"type\":\"string\",\"enum\":[\"A\",\"B\"]}","buttons":[{"label":"A","value":"A"}]}
Multi: {"name":"items","question":"Select:","schema":"{\"type\":\"array\",\"items\":{\"type\":\"string\",\"enum\":[\"A\",\"B\"]}}"}}"#
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "Array of questions (always array, single = 1-item array)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "question": {"type": "string"},
                            "schema": {"type": "string"},
                            "buttons": {
                                "type": ["array", "null"],
                                "description": "Optional button labels for single-select questions (string enum). Do NOT provide for array/checkbox questions.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {"type": "string"},
                                        "value": {
                                            "type": ["string", "number", "boolean"],
                                            "description": "Button value (simple types: string, number, or boolean)"
                                        },
                                        "variant": {"type": ["string", "null"]}
                                    },
                                    "required": ["label", "value"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["name", "question", "schema"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["questions"],
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _conn: &mut DbConn,
        _storage: &FileStorageService,
        _workspace_id: Uuid,
        _user_id: Uuid,
        _config: ToolConfig,
        args: Value,
    ) -> Result<ToolResponse> {
        let ask_args: AskUserArgs = serde_json::from_value(args)?;

        // Validate questions array
        if ask_args.questions.is_empty() {
            return Err(Error::Validation(crate::error::ValidationErrors::Single {
                field: "questions".to_string(),
                message: "questions array cannot be empty".to_string(),
            }));
        }

        // Generate time-ordered unique question ID using UUID v7
        // This provides better debugging and logging capabilities compared to random v4 IDs
        let question_id = Uuid::now_v7();

        // Convert QuestionInput to SSE Question format
        let questions: Vec<crate::models::sse::Question> = ask_args.questions
            .into_iter()
            .map(|q| {
                crate::models::sse::Question {
                    name: q.name,
                    question: q.question,
                    schema: q.schema.0,
                    buttons: q.buttons.map(|btns| btns.into_iter().map(|b| {
                        crate::models::sse::QuestionButton {
                            label: b.label,
                            value: b.value.0,
                            variant: b.variant,
                        }
                    }).collect()),
                }
            })
            .collect();

        // Return result with question_pending status and questions
        let result = AskUserResult {
            status: "question_pending".to_string(),
            question_id,
            questions,
        };

        Ok(ToolResponse {
            success: true,
            result: serde_json::to_value(result)?,
            error: None,
        })
    }
}
