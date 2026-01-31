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
        r#"Request input or confirmation from the user. Supports multiple questions in one call. Questions are ephemeral (SSE-only, not persisted).

=== WHEN TO USE ===
1. **Clarification Needed**: User's request is ambiguous or incomplete
2. **Multiple Valid Approaches**: Several options exist and user preference matters
3. **Confirmation Required**: Action is significant or irreversible (deletion, major changes)
4. **Missing Information**: You need specific details to proceed
5. **Design Decisions**: User's input affects the outcome

=== BUTTONS VS CHECKBOXES ===

**Single-Select Questions** (Use buttons):
- Schema: {"type": "string", "enum": ["A", "B", "C"]}
- Add buttons field for better UX
- Example:
```json
{
  "name": "choice",
  "question": "Which approach do you prefer?",
  "schema": "{\"type\":\"string\",\"enum\":[\"Option A\",\"Option B\",\"Option C\"]}",
  "buttons": [
    {"label": "Option A", "value": "\"A\""},
    {"label": "Option B", "value": "\"B\""},
    {"label": "Option C", "value": "\"C\""}
  ]
}
```

**Multi-Select Questions** (NO buttons!):
- Schema: {"type": "array", "items": {"type": "string", "enum": ["A", "B", "C"]}}
- DO NOT add buttons field - frontend will render checkboxes automatically
- Example:
```json
{
  "name": "choices",
  "question": "Select all that apply:",
  "schema": "{\"type\":\"array\",\"items\":{\"type\":\"string\",\"enum\":[\"A\",\"B\",\"C\"]},\"minItems\":1}"
  // NO buttons field! Frontend handles this.
}
```

=== COMMON SCHEMAS ===

String with enum (radio buttons):
  {"type":"string","enum":["Yes","No","Cancel"]}

String with pattern (text input):
  {"type":"string","pattern":"^[a-z0-9]+$","minLength":3}

Number with range:
  {"type":"number","minimum":1,"maximum":100}

Array/checkbox (multi-select):
  {"type":"array","items":{"type":"string","enum":["A","B","C"]},"minItems":1}

=== CRITICAL RULES ===
- NEVER provide `buttons` for array-type questions (type: "array")
- ALWAYS provide `buttons` for string enum questions (better UX)
- Make questions specific and concise
- Provide context in the question text
- Use "Select one" or "choose all that apply" suffix to indicate selection type"#
    }

    fn definition(&self) -> Value {
        super::strict_tool_schema::<AskUserArgs>()
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

        // Generate unique question ID
        // TODO: Use UUID v7 for time-ordered when timestamp is available
        let question_id = Uuid::new_v4();

        // Convert QuestionInput to SSE Question format
        let questions: Vec<crate::models::sse::Question> = ask_args.questions
            .into_iter()
            .map(|q| {
                // Parse schema JSON string to Value
                let schema_value: serde_json::Value = serde_json::from_str(&q.schema)
                    .unwrap_or_else(|e| {
                        tracing::warn!(error = %e, schema = %q.schema, "Failed to parse question schema, using empty object");
                        serde_json::json!({})
                    });

                crate::models::sse::Question {
                    name: q.name,
                    question: q.question,
                    schema: schema_value,
                    buttons: q.buttons.map(|btns| btns.into_iter().map(|b| {
                        // Parse value JSON string to Value
                        let value_value: serde_json::Value = serde_json::from_str(&b.value)
                            .unwrap_or_else(|e| {
                                tracing::warn!(error = %e, value = %b.value, "Failed to parse button value, using raw string");
                                serde_json::json!(b.value)
                            });

                        crate::models::sse::QuestionButton {
                            label: b.label,
                            value: value_value,
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
