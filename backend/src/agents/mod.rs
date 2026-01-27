pub mod assistant;

/// Central registry for agent personas.
///
/// Returns the system prompt for a given role, falling back to the
/// default Assistant persona if the role is not recognized or provided.
pub fn get_persona(role: Option<&str>) -> String {
    match role {
        // Future roles like "architect", "coder", "blogger" can be added here
        _ => assistant::SYSTEM_PROMPT.to_string(),
    }
}
