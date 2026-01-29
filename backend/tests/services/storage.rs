//! Tests for FileStorageService, focusing on security and correctness

use buildscale::services::storage::FileStorageService;
use uuid::Uuid;

#[tokio::test]
async fn test_read_file_rejects_parent_directory_traversal() {
    let storage = FileStorageService::new("/tmp/test_storage");
    let workspace_id = Uuid::new_v4();

    // Test various path traversal attempts through read_file
    let traversal_attempts = vec![
        "../../etc/passwd",
        "../sensitive_file",
        "folder/../../etc/passwd",
        "./../etc/passwd",
        "normal_folder/../../escape",
    ];

    for path in traversal_attempts {
        let result = storage.read_file(workspace_id, path).await;
        assert!(
            result.is_err(),
            "Path '{}' should be rejected due to parent directory reference",
            path
        );
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("Path cannot contain") || error_msg.contains("VALIDATION_ERROR"),
                "Error message should mention path validation, got: {}",
                error_msg
            );
        }
    }
}

#[tokio::test]
async fn test_read_file_allows_valid_absolute_paths() {
    let storage = FileStorageService::new("/tmp/test_storage");
    let workspace_id = Uuid::new_v4();

    // Absolute paths are allowed (they're normalized to relative)
    // This is valid because they stay within the workspace
    let valid_absolute_paths = vec![
        "/notes/file.txt",
        "/folder/subfolder/file.md",
        "/chats/chat-123",
    ];

    for path in valid_absolute_paths {
        // These won't exist (NotFound), but they should pass path validation
        let result = storage.read_file(workspace_id, path).await;
        // We expect NotFound (file doesn't exist), NOT ValidationError
        match result {
            Err(e) => {
                let error_msg = format!("{}", e);
                assert!(
                    !error_msg.contains("VALIDATION_ERROR"),
                    "Valid path '{}' should not trigger validation error, got: {}",
                    path, error_msg
                );
            }
            Ok(_) => {} // File exists, that's fine too
        }
    }
}

#[tokio::test]
async fn test_write_file_rejects_malicious_paths() {
    let storage = FileStorageService::new("/tmp/test_storage");
    let workspace_id = Uuid::new_v4();
    let content = b"test content";

    // Test that write_file also rejects malicious paths
    let malicious_paths = vec![
        "../../etc/passwd",
        "/../../../etc/passwd",
        "../secret",
    ];

    for path in malicious_paths {
        let result = storage.write_latest_file(workspace_id, path, content).await;
        assert!(
            result.is_err(),
            "Malicious path '{}' should be rejected by write_latest_file",
            path
        );
    }
}

#[tokio::test]
async fn test_workspace_isolation() {
    let storage = FileStorageService::new("/tmp/test_storage");
    let workspace1 = Uuid::new_v4();
    let workspace2 = Uuid::new_v4();

    // Write a file to workspace1
    let content1 = b"workspace 1 data";
    let result = storage.write_latest_file(workspace1, "safe_file.txt", content1).await;
    assert!(result.is_ok(), "Should successfully write to workspace1");

    // Try to access workspace1's file from workspace2 using traversal
    // This won't work because we validate the path, but let's test anyway
    let traversal_path = format!("../{}/safe_file.txt", workspace1);
    let traversal_result = storage.read_file(workspace2, &traversal_path).await;

    // The traversal attempt should be rejected
    assert!(traversal_result.is_err(), "Traversal between workspaces should be blocked");
}

#[tokio::test]
async fn test_prevents_complex_escape_sequences() {
    let storage = FileStorageService::new("/tmp/test_storage");
    let workspace_id = Uuid::new_v4();

    // Try to escape using various obfuscation techniques
    let escape_attempts = vec![
        "....//....//etc/passwd",
        "..//..//etc/passwd",
        "....\\....\\windows\\system32",  // Windows-style obfuscated traversal
    ];

    for path in escape_attempts {
        let result = storage.read_file(workspace_id, path).await;
        assert!(
            result.is_err(),
            "Obfuscated escape attempt '{}' should be rejected",
            path
        );
    }
}

