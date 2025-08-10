/// Simple template engine for ESP32 with minimal memory usage
use std::collections::HashMap;

/// Basic template engine that supports variable substitution
pub struct TemplateEngine;

impl TemplateEngine {
    /// Render a template with variables
    /// Variables are in the format {{variable_name}}
    pub fn render(template: &str, vars: &HashMap<&str, String>) -> String {
        let mut result = String::from(template);
        
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }
    
    /// Render with partials support (include other templates)
    /// Partials are in the format {{>partial_name}}
    pub fn render_with_partials(
        template: &str, 
        vars: &HashMap<&str, String>,
        partials: &HashMap<&str, &str>
    ) -> String {
        let mut result = String::from(template);
        
        // First process partials
        for (name, content) in partials {
            let placeholder = format!("{{{{>{}}}}}", name);
            result = result.replace(&placeholder, content);
        }
        
        // Then process variables
        Self::render(&result, vars)
    }

    /// Render with partials and additional string flags (e.g., active navbar classes)
    pub fn render_with_partials_and_flags(
        template: &str,
        vars: &HashMap<&str, String>,
        partials: &HashMap<&str, &str>,
        flags: &HashMap<&str, &str>
    ) -> String {
        let mut result = Self::render_with_partials(template, vars, partials);
        for (key, value) in flags {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}