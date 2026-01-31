//! Unit tests for Plan Mode functionality
//!
//! Tests for ToolConfig, plan mode guards, and mode-based tool restrictions.

use buildscale::tools::ToolConfig;

#[test]
fn test_toolconfig_default() {
    let config = ToolConfig::default();
    // Default is Build Mode (plan_mode: false) for backward compatibility with tests
    assert!(!config.plan_mode, "Default should be build mode");
    assert!(config.active_plan_path.is_none(), "Default should have no active plan");
}

#[test]
fn test_toolconfig_build_mode() {
    let config = ToolConfig {
        plan_mode: false,
        active_plan_path: Some("/plans/my-plan.plan".to_string()),
    };
    assert!(!config.plan_mode, "Should be in build mode");
    assert_eq!(
        config.active_plan_path,
        Some("/plans/my-plan.plan".to_string()),
        "Should have active plan path"
    );
}

#[test]
fn test_toolconfig_plan_mode() {
    let config = ToolConfig {
        plan_mode: true,
        active_plan_path: None,
    };
    assert!(config.plan_mode, "Should be in plan mode");
    assert!(config.active_plan_path.is_none(), "Should have no active plan in plan mode");
}
