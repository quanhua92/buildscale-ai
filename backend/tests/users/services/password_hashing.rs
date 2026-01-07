use buildscale::services::users::{generate_password_hash, verify_password};

#[test]
fn test_generate_password_hash_basic() {
    // Test basic password hashing
    let password = "testpassword123";
    let result = generate_password_hash(password);

    assert!(result.is_ok(), "Password hashing should succeed");

    let hash = result.unwrap();
    assert!(!hash.is_empty(), "Hash should not be empty");
    assert!(
        hash.starts_with("$argon2"),
        "Hash should start with Argon2 identifier"
    );
    assert!(hash.len() > 50, "Hash should be substantial length");
}

#[test]
fn test_generate_password_hash_different_passwords() {
    // Test that different passwords produce different hashes
    let password1 = "SecurePass123!";
    let password2 = "different456";

    let hash1 = generate_password_hash(password1).unwrap();
    let hash2 = generate_password_hash(password2).unwrap();

    assert_ne!(
        hash1, hash2,
        "Different passwords should produce different hashes"
    );

    // Verify hashes are valid Argon2 format
    assert!(hash1.starts_with("$argon2"));
    assert!(hash2.starts_with("$argon2"));
}

#[test]
fn test_generate_password_hash_same_password_different_hashes() {
    // Test that the same password produces different hashes due to salt
    let password = "samepassword123";

    let hash1 = generate_password_hash(password).unwrap();
    let hash2 = generate_password_hash(password).unwrap();

    assert_ne!(
        hash1, hash2,
        "Same password should produce different hashes due to random salt"
    );

    // Both should be valid Argon2 hashes
    assert!(hash1.starts_with("$argon2"));
    assert!(hash2.starts_with("$argon2"));

    // But they should both verify with the original password
    assert!(verify_password(password, &hash1).unwrap());
    assert!(verify_password(password, &hash2).unwrap());
}

#[test]
fn test_generate_password_hash_various_lengths() {
    // Test passwords of various valid lengths
    let test_cases = vec![
        "valid8",
        "validpass10",
        "veryveryverylongpassword",
        "complex!@#$%^789",
        "UPPERCASE123",
        "lowercase456",
        "MixedCase123!@#",
    ];

    for password in test_cases {
        let hash = generate_password_hash(password).unwrap();
        assert!(
            hash.starts_with("$argon2"),
            "Password '{}' should produce valid Argon2 hash",
            password
        );
        assert!(!hash.is_empty(), "Hash should not be empty");
    }
}

#[test]
fn test_generate_password_hash_special_characters() {
    // Test passwords with various special characters
    let special_passwords = vec![
        "password!@#$%^&*()",
        "pass_with_underscores",
        "pass-with-dashes",
        "pass.with.dots",
        "pass with spaces",
        "密码123", // Chinese characters
        "motdepasse123", // French
        "пароль123", // Russian
        "パスワード123", // Japanese
    ];

    for password in special_passwords {
        let hash = generate_password_hash(password).unwrap();
        assert!(
            hash.starts_with("$argon2"),
            "Password with special chars '{}' should produce valid hash",
            password
        );

        // Verify the hash works with verify_password
        let is_valid = verify_password(password, &hash).unwrap();
        assert!(is_valid, "Hash should verify correctly for password");
    }
}

#[test]
fn test_generate_password_hash_consistency() {
    // Test that generated hashes are always in the expected format
    let password = "consistency_test";

    for _ in 0..10 {
        let hash = generate_password_hash(password).unwrap();

        // Check Argon2 format: $argon2$version=...$params=...$hash
        assert!(hash.starts_with("$argon2"), "Hash should start with $argon2");
        assert!(hash.len() > 50, "Hash should be substantial length");

        // Should contain multiple $ separators
        let parts: Vec<&str> = hash.split('$').collect();
        assert!(parts.len() >= 5, "Hash should have proper Argon2 structure");

        // Verify it can be used with verify_password
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }
}

#[test]
fn test_generate_password_hash_empty_password() {
    // Test that empty passwords can still be hashed
    let password = "";
    let result = generate_password_hash(password);

    assert!(result.is_ok(), "Empty password should still be hashable");

    let hash = result.unwrap();
    assert!(hash.starts_with("$argon2"), "Empty password hash should be valid Argon2");

    // Verify empty password works with verification
    assert!(verify_password(password, &hash).unwrap());
    assert!(!verify_password("notempty", &hash).unwrap());
}

#[test]
fn test_generate_password_hash_unicode() {
    // Test passwords with Unicode characters
    let unicode_passwords = vec![
        "🔐🔑🗝️", // Emojis
        "café123", // Accented characters
        "naïve456", // Multiple diacritics
        "привет789", // Cyrillic
        "こんにちは", // Hiragana
        "안녕하세요", // Korean
        "العربية", // Arabic
        "עברית", // Hebrew
    ];

    for password in unicode_passwords {
        let hash = generate_password_hash(password).unwrap();
        assert!(
            hash.starts_with("$argon2"),
            "Unicode password should produce valid hash"
        );

        // Verify the hash works correctly
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }
}