pub mod common;
pub mod assistant;
pub mod planner;
pub mod builder;

/// Central registry for agent personas.
///
/// Returns the system prompt for a given role, falling back to the
/// default Assistant persona if the role is not recognized or provided.
///
/// # Arguments
/// * `role` - Optional role identifier (e.g., "planner", "builder")
/// * `mode` - Optional chat mode ("plan" or "build") to auto-select persona
/// * `plan_content` - Optional plan content for Build Mode (required when role="builder")
///
/// # Returns
/// System prompt string for the requested persona
///
/// # Examples
/// ```
/// use buildscale::agents;
///
/// // Get planner persona
/// let prompt = agents::get_persona(Some("planner"), None, None);
///
/// // Get builder persona with plan content
/// let prompt = agents::get_persona(Some("builder"), None, Some("Plan content here"));
///
/// // Auto-select based on mode
/// let prompt = agents::get_persona(None, Some("plan"), None);
///
/// // Default to assistant
/// let prompt = agents::get_persona(Some("unknown"), None, None);
/// ```
pub fn get_persona(role: Option<&str>, mode: Option<&str>, plan_content: Option<&str>) -> String {
    // Priority: explicit role > mode-based selection > default
    match role {
        Some("builder") => {
            // Builder requires plan content
            let plan = plan_content.unwrap_or("# No Plan Provided\n\nError: Builder agent requires a plan.");
            builder::get_system_prompt(plan)
        }
        Some("planner") => planner::get_system_prompt(),
        None => {
            // Mode-based selection
            match mode {
                Some("plan") => planner::get_system_prompt(),
                Some("build") => {
                    // Builder persona requires plan content
                    // This is handled by ChatService::build_context which reads the plan file
                    let plan = plan_content.unwrap_or("# No Plan Provided\n\nError: Builder agent requires a plan.");
                    builder::get_system_prompt(plan)
                }
                None => assistant::get_system_prompt(),
                _ => assistant::get_system_prompt(),
            }
        }
        _ => assistant::get_system_prompt(),
    }
}
