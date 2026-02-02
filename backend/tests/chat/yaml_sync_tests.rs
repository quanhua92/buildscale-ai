//! Unit tests for YAML frontmatter sync functionality
//!
//! Tests for ChatFrontmatter, YamlFrontmatter, and sync utilities.

use buildscale::services::chat::{ChatFrontmatter, YamlFrontmatter};
use buildscale::models::chat::AgentConfig;

#[test]
fn test_chat_frontmatter_from_agent_config() {
    let config = AgentConfig {
        agent_id: None,
        model: "gpt-5-mini".to_string(),
        temperature: 0.7,
        persona_override: None,
        previous_response_id: None,
        mode: "plan".to_string(),
        plan_file: None,
    };

    let frontmatter = ChatFrontmatter::from_agent_config(&config);
    assert_eq!(frontmatter.mode, "plan");
    assert!(frontmatter.plan_file.is_none());
}

#[test]
fn test_chat_frontmatter_with_plan_file() {
    let config = AgentConfig {
        agent_id: None,
        model: "gpt-5-mini".to_string(),
        temperature: 0.7,
        persona_override: None,
        previous_response_id: None,
        mode: "build".to_string(),
        plan_file: Some("/plans/my-plan.plan".to_string()),
    };

    let frontmatter = ChatFrontmatter::from_agent_config(&config);
    assert_eq!(frontmatter.mode, "build");
    assert_eq!(frontmatter.plan_file, Some("/plans/my-plan.plan".to_string()));
}

#[test]
fn test_yaml_frontmatter_parse_with_frontmatter() {
    let content = r#"---
mode: build
plan_file: /plans/example.plan
---
Some chat content here"#;

    let parsed = YamlFrontmatter::parse(content).unwrap();
    assert_eq!(parsed.frontmatter.mode, "build");
    assert_eq!(parsed.frontmatter.plan_file, Some("/plans/example.plan".to_string()));
    assert_eq!(parsed.content.trim(), "Some chat content here");
}

#[test]
fn test_yaml_frontmatter_parse_without_frontmatter() {
    let content = "Just regular content without frontmatter";

    let parsed = YamlFrontmatter::parse(content).unwrap();
    assert_eq!(parsed.frontmatter.mode, "plan"); // Default
    assert!(parsed.frontmatter.plan_file.is_none());
    assert_eq!(parsed.content, content);
}

#[test]
fn test_yaml_frontmatter_serialize_and_parse() {
    let frontmatter = ChatFrontmatter {
        mode: "build".to_string(),
        plan_file: Some("/plans/test.plan".to_string()),
        model: None,
        extra: std::collections::HashMap::new(),
    };

    let yaml_frontmatter = YamlFrontmatter::new(frontmatter, "Chat content".to_string());
    let serialized = yaml_frontmatter.serialize().unwrap();
    let reparsed = YamlFrontmatter::parse(&serialized).unwrap();

    assert_eq!(yaml_frontmatter.frontmatter.mode, reparsed.frontmatter.mode);
    assert_eq!(
        yaml_frontmatter.frontmatter.plan_file,
        reparsed.frontmatter.plan_file
    );
    assert_eq!(reparsed.content.trim(), "Chat content");
}

#[test]
fn test_chat_frontmatter_merge_into_agent_config() {
    let frontmatter = ChatFrontmatter {
        mode: "build".to_string(),
        plan_file: Some("/plans/merge-test.plan".to_string()),
        model: None,
        extra: std::collections::HashMap::new(),
    };

    let config = AgentConfig {
        agent_id: Some(uuid::Uuid::new_v4()),
        model: "gpt-5".to_string(),
        temperature: 0.8,
        persona_override: Some("Custom persona".to_string()),
        previous_response_id: Some("response-123".to_string()),
        mode: "plan".to_string(), // This should be overridden
        plan_file: None,          // This should be overridden
    };

    let merged = frontmatter.merge_into_agent_config(config);

    assert_eq!(merged.mode, "build", "Mode should be updated");
    assert_eq!(
        merged.plan_file,
        Some("/plans/merge-test.plan".to_string()),
        "Plan file should be updated"
    );
    assert_eq!(
        merged.model, "gpt-5",
        "Other fields should be preserved"
    );
    assert_eq!(merged.temperature, 0.8, "Temperature should be preserved");
}
