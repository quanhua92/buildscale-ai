/// Test that verifies the fix for AI confusion with boolean parameters in tool calls.
///
/// This test verifies that:
/// 1. Core tools have compact descriptions (for token efficiency)
/// 2. Boolean parameters have explicit type hints in schemas (for AI guidance)
#[cfg(test)]
mod tests {
    /// Test that ls tool description is compact and includes key functionality
    #[test]
    fn test_ls_tool_description_includes_examples() {
        use buildscale::tools::ls::LsTool;
        use buildscale::tools::Tool;

        let tool = LsTool;
        let description = tool.description();

        println!("LS tool description:\n{}", description);

        // Verify the description is compact and describes core functionality
        // (Descriptions are now compacted for token efficiency - examples are in schemas)
        assert!(description.contains("Lists directory contents"), "Description should describe functionality");
        assert!(description.len() < 300, "Description should be compact (under 300 chars)");
    }

    /// Test that ls tool schema has explicit boolean type hints
    #[test]
    fn test_ls_tool_schema_has_boolean_hints() {
        use buildscale::tools::ls::LsTool;
        use buildscale::tools::Tool;

        let tool = LsTool;
        let schema = tool.definition();

        println!("LS tool schema:\n{}", serde_json::to_string_pretty(&schema).unwrap());

        // Check that recursive parameter has explicit description
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(recursive) = props.get("recursive") {
                if let Some(desc) = recursive.get("description").and_then(|d| d.as_str()) {
                    println!("Recursive parameter description: {}", desc);
                    assert!(
                        desc.contains("JSON boolean") && desc.contains("true/false"),
                        "Schema should explicitly mention JSON boolean format"
                    );
                } else {
                    panic!("Recursive parameter should have a description field");
                }
            } else {
                panic!("Schema should have recursive parameter");
            }
        } else {
            panic!("Schema should have properties");
        }
    }

    /// Test that grep tool schema has explicit boolean type hints
    #[test]
    fn test_grep_tool_schema_has_boolean_hints() {
        use buildscale::tools::grep::GrepTool;
        use buildscale::tools::Tool;

        let tool = GrepTool;
        let schema = tool.definition();

        println!("Grep tool schema:\n{}", serde_json::to_string_pretty(&schema).unwrap());

        // Check that case_sensitive parameter has explicit description
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(case_sensitive) = props.get("case_sensitive") {
                if let Some(desc) = case_sensitive.get("description").and_then(|d| d.as_str()) {
                    println!("case_sensitive parameter description: {}", desc);
                    assert!(
                        desc.contains("JSON boolean") || desc.contains("true/false"),
                        "Schema should explicitly mention JSON boolean format"
                    );
                } else {
                    panic!("case_sensitive parameter should have a description field");
                }
            } else {
                panic!("Schema should have case_sensitive parameter");
            }
        } else {
            panic!("Schema should have properties");
        }
    }

    /// Test that write tool schema has explicit boolean type hints
    #[test]
    fn test_write_tool_schema_has_boolean_hints() {
        use buildscale::tools::write::WriteTool;
        use buildscale::tools::Tool;

        let tool = WriteTool;
        let schema = tool.definition();

        println!("Write tool schema:\n{}", serde_json::to_string_pretty(&schema).unwrap());

        // Check that overwrite parameter has explicit description
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(overwrite) = props.get("overwrite") {
                if let Some(desc) = overwrite.get("description").and_then(|d| d.as_str()) {
                    println!("overwrite parameter description: {}", desc);
                    assert!(
                        desc.contains("JSON boolean") || desc.contains("true/false"),
                        "Schema should explicitly mention JSON boolean format"
                    );
                } else {
                    panic!("overwrite parameter should have a description field");
                }
            } else {
                panic!("Schema should have overwrite parameter");
            }
        } else {
            panic!("Schema should have properties");
        }
    }
}
