#[cfg(test)]
mod tests {
    use crate::services::chat::context::{
        AttachmentKey, AttachmentManager, AttachmentValue, PRIORITY_ESSENTIAL, PRIORITY_HIGH,
        PRIORITY_LOW, PRIORITY_MEDIUM,
    };
    use uuid::Uuid;

    #[test]
    fn test_attachment_ordering() {
        let mut manager = AttachmentManager::new();

        // Add fragments out of order
        manager.add_fragment(
            AttachmentKey::UserRequest,
            AttachmentValue {
                content: "User Request".to_string(),
                priority: 1,
                tokens: 10,
                is_essential: true,
            },
        );
        manager.add_fragment(
            AttachmentKey::SystemPersona,
            AttachmentValue {
                content: "System Persona".to_string(),
                priority: 0,
                tokens: 10,
                is_essential: true,
            },
        );
        manager.add_fragment(
            AttachmentKey::Environment,
            AttachmentValue {
                content: "Environment".to_string(),
                priority: 2,
                tokens: 10,
                is_essential: false,
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

        // Essential fragment
        manager.add_fragment(
            AttachmentKey::SystemPersona,
            AttachmentValue {
                content: "Essential".to_string(),
                priority: PRIORITY_ESSENTIAL,
                tokens: 100,
                is_essential: true,
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

        manager.add_fragment(
            AttachmentKey::WorkspaceFile(file_id),
            AttachmentValue {
                content: "Version 1".to_string(),
                priority: PRIORITY_HIGH,
                tokens: 10,
                is_essential: false,
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
}
