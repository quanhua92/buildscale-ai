#[cfg(test)]
mod tests {
    use crate::services::chat::context::{
        AttachmentKey, AttachmentManager, AttachmentValue, PRIORITY_ESSENTIAL, PRIORITY_HIGH,
        PRIORITY_LOW, PRIORITY_MEDIUM, ESTIMATED_CHARS_PER_TOKEN, truncate_tool_output,
    };
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_attachment_ordering() {
        let mut manager = AttachmentManager::new();
        let now = Utc::now();

        // Add fragments out of order
        manager.add_fragment(
            AttachmentKey::UserRequest,
            AttachmentValue {
                content: "User Request".to_string(),
                priority: 1,
                tokens: 10,
                is_essential: true,
                created_at: now,
                updated_at: None,
            },
        );
        manager.add_fragment(
            AttachmentKey::SystemPersona,
            AttachmentValue {
                content: "System Persona".to_string(),
                priority: 0,
                tokens: 10,
                is_essential: true,
                created_at: now,
                updated_at: None,
            },
        );
        manager.add_fragment(
            AttachmentKey::Environment,
            AttachmentValue {
                content: "Environment".to_string(),
                priority: 2,
                tokens: 10,
                is_essential: false,
                created_at: now,
                updated_at: None,
            },
        );

        manager.sort_by_position();

        let rendered = manager.render();
        let lines: Vec<&str> = rendered.split("\n\n").collect();

        assert_eq!(lines[0], "System Persona");
        assert_eq!(lines[1], "Environment");
        assert_eq!(lines[2], "User Request");
    }

    #[test]
    fn test_token_pruning() {
        let mut manager = AttachmentManager::new();
        let now = Utc::now();

        // Essential fragment
        manager.add_fragment(
            AttachmentKey::SystemPersona,
            AttachmentValue {
                content: "Essential".to_string(),
                priority: PRIORITY_ESSENTIAL,
                tokens: 100,
                is_essential: true,
                created_at: now,
                updated_at: None,
            },
        );

        // Non-essential, low priority (high value)
        manager.add_fragment(
            AttachmentKey::ChatHistory,
            AttachmentValue {
                content: "Old History".to_string(),
                priority: PRIORITY_LOW,
                tokens: 200,
                is_essential: false,
                created_at: now,
                updated_at: None,
            },
        );

        // Non-essential, medium priority
        let file_id = Uuid::now_v7();
        manager.add_fragment(
            AttachmentKey::WorkspaceFile(file_id),
            AttachmentValue {
                content: "Some File".to_string(),
                priority: PRIORITY_MEDIUM,
                tokens: 150,
                is_essential: false,
                created_at: now,
                updated_at: None,
            },
        );

        // Current total tokens: 100 + 200 + 150 = 450
        // Limit to 300: Should drop Old History (450 - 200 = 250)
        manager.optimize_for_limit(300);

        assert!(manager.map.contains_key(&AttachmentKey::SystemPersona));
        assert!(manager
            .map
            .contains_key(&AttachmentKey::WorkspaceFile(file_id)));
        assert!(!manager.map.contains_key(&AttachmentKey::ChatHistory));
    }

    #[test]
    fn test_fragment_replacement() {
        let mut manager = AttachmentManager::new();
        let file_id = Uuid::now_v7();
        let now = Utc::now();

        manager.add_fragment(
            AttachmentKey::WorkspaceFile(file_id),
            AttachmentValue {
                content: "Version 1".to_string(),
                priority: PRIORITY_HIGH,
                tokens: 10,
                is_essential: false,
                created_at: now,
                updated_at: None,
            },
        );

        // Replace with Version 2
        manager.add_fragment(
            AttachmentKey::WorkspaceFile(file_id),
            AttachmentValue {
                content: "Version 2".to_string(),
                priority: PRIORITY_HIGH,
                tokens: 15,
                is_essential: false,
                created_at: now,
                updated_at: None,
            },
        );

        assert_eq!(manager.map.len(), 1);
        assert_eq!(
            manager
                .map
                .get(&AttachmentKey::WorkspaceFile(file_id))
                .unwrap()
                .content,
            "Version 2"
        );
    }

    #[test]
    fn test_tool_output_summarization_read() {
        use crate::services::chat::ChatService;

        // Generate 1000 real lines as plain text (simulating non-JSON fallback case)
        let mut long_content = String::new();
        for i in 1..=1000 {
            long_content.push_str(&format!("This is line number {}\n", i));
        }

        // Ensure it exceeds the 2048 byte limit for summarization to trigger
        assert!(long_content.len() > 2048);

        let summarized = ChatService::summarize_tool_outputs("read", &long_content);

        // When non-JSON is passed, fallback_line_truncation is used
        // Format: "{preview}\n... ({N} more lines)"
        // Verify it contains the truncation indicator
        assert!(summarized.contains("... ("));
        assert!(summarized.contains("more lines)"));

        // Verify it contains the first 20 lines (CONTENT_PREVIEW_LINES) in the preview
        assert!(summarized.contains("This is line number 1"));
        assert!(summarized.contains("This is line number 20"));

        // Verify it does NOT contain the 21st line in the preview
        assert!(!summarized.contains("This is line number 21\n"));
    }

    #[test]
    fn test_tool_input_summarization_write() {
        use crate::services::chat::ChatService;

        // Use content with many lines to ensure truncation triggers
        let mut long_content = String::new();
        for i in 1..=1000 {
            long_content.push_str(&format!("This is line number {}\n", i));
        }

        let args = serde_json::json!({
            "path": "/test.txt",
            "content": long_content
        });

        let summarized = ChatService::summarize_tool_inputs("write", &args);
        let content = summarized.get("content").unwrap().as_str().unwrap();

        // Should contain a preview and the truncation stats with line count
        assert!(content.contains("... [truncated,"));
        assert!(content.contains("lines total]"));
        // The original is roughly 25,000 chars, summarized should be much smaller
        assert!(content.len() < 1000);
    }

    #[test]
    fn test_tool_input_summarization_edit() {
        use crate::services::chat::ChatService;

        // Use content with many lines (line-based truncation is used now)
        let mut long_string = String::new();
        for i in 1..=100 {
            long_string.push_str(&format!("word{}\n", i));
        }

        let args = serde_json::json!({
            "path": "/test.txt",
            "old_string": long_string.clone(),
            "new_string": "short"
        });

        let summarized = ChatService::summarize_tool_inputs("edit", &args);
        let old_string = summarized.get("old_string").unwrap().as_str().unwrap();
        let new_string = summarized.get("new_string").unwrap().as_str().unwrap();

        // New format includes line count
        assert!(old_string.contains("... [truncated,"));
        assert!(old_string.contains("lines]"));
        assert_eq!(new_string, "short");
    }

    /// Regression test: Token count should be calculated from TRUNCATED content,
    /// not original content. This prevents showing inflated token counts in the
    /// context UI for old tool results that have been truncated.
    ///
    /// Bug: Previously, token_count was calculated from original_length instead of
    /// truncated content length, causing a 12,000 char tool result to show 3,000 tokens
    /// even though only ~50 characters were actually used after truncation.
    #[test]
    fn test_token_count_uses_truncated_content_not_original() {
        use crate::models::chat::{ChatMessage, ChatMessageMetadata, ChatMessageRole};
        use crate::services::chat::context::KEEP_RECENT_TOOL_RESULTS;
        use crate::services::chat::ChatService;

        // 1. Create a long tool result message that will be truncated.
        let original_content = "x".repeat(12000);
        let file_id = Uuid::now_v7();
        let workspace_id = Uuid::now_v7();
        let long_tool_msg = ChatMessage {
            id: Uuid::now_v7(),
            file_id,
            workspace_id,
            role: ChatMessageRole::Tool,
            content: original_content.clone(),
            metadata: sqlx::types::Json(ChatMessageMetadata {
                message_type: Some("tool_result".to_string()),
                ..Default::default()
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        // 2. Create enough recent tool results to push the long one into truncation territory.
        let mut messages = vec![long_tool_msg.clone()];
        for i in 0..KEEP_RECENT_TOOL_RESULTS {
            messages.push(ChatMessage {
                id: Uuid::now_v7(),
                file_id: long_tool_msg.file_id,
                workspace_id: long_tool_msg.workspace_id,
                role: ChatMessageRole::Tool,
                content: format!("short result {}", i),
                metadata: sqlx::types::Json(ChatMessageMetadata {
                    message_type: Some("tool_result".to_string()),
                    ..Default::default()
                }),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deleted_at: None,
            });
        }

        // 3. Call the function under test.
        let attachment_manager = AttachmentManager::new();
        let history_section = ChatService::build_history_section(&messages, &attachment_manager);

        // 4. Find our message in the result and assert on its token count.
        let info = history_section
            .messages
            .iter()
            .find(|m| m.content_length == original_content.len())
            .expect("Long message not found in history section");

        let truncated_content = truncate_tool_output(&original_content);
        let correct_token_count = truncated_content.len() / ESTIMATED_CHARS_PER_TOKEN;

        assert_eq!(
            info.token_count, correct_token_count,
            "Token count should be based on truncated content"
        );
        assert_eq!(
            info.content_length,
            original_content.len(),
            "Content length should be the original length"
        );
    }
}
