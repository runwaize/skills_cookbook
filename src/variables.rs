use crate::error::{RelayError, Result};
use crate::types::{ResolvedVariable, VariableSchema, VariableScope, VariableSource};
use std::env;

#[allow(dead_code)]
pub struct VariableResolver {
    workspace_id: Option<String>,
    project_id: Option<String>,
}

#[allow(dead_code)]
impl VariableResolver {
    pub fn new(workspace_id: Option<String>, project_id: Option<String>) -> Self {
        Self {
            workspace_id,
            project_id,
        }
    }

    /// Resolve variables for a skill based on schema
    pub async fn resolve_variables(
        &self,
        schemas: &[VariableSchema],
    ) -> Result<Vec<ResolvedVariable>> {
        let mut resolved = Vec::new();

        for schema in schemas {
            match self.resolve_variable(schema).await {
                Ok(var) => resolved.push(var),
                Err(e) => {
                    if schema.required {
                        return Err(RelayError::VariableResolution(format!(
                            "Required variable '{}' could not be resolved: {}",
                            schema.name, e
                        )));
                    } else {
                        tracing::warn!(
                            "Optional variable '{}' could not be resolved: {}",
                            schema.name,
                            e
                        );
                    }
                }
            }
        }

        Ok(resolved)
    }

    /// Returns names of required variables that cannot be resolved (no secret values).
    pub async fn missing_required_variables(
        &self,
        schemas: &[VariableSchema],
    ) -> Vec<String> {
        let mut missing = Vec::new();
        for schema in schemas {
            if !schema.required {
                continue;
            }
            if self.resolve_variable(schema).await.is_err() {
                missing.push(schema.name.clone());
            }
        }
        missing
    }

    /// Resolve a single variable
    async fn resolve_variable(&self, schema: &VariableSchema) -> Result<ResolvedVariable> {
        // Try resolution chain based on scope
        let (value, source) = match schema.scope {
            VariableScope::Personal => self.resolve_personal_variable(schema).await?,
            VariableScope::Workspace => self.resolve_workspace_variable(schema).await?,
            VariableScope::Project => self.resolve_project_variable(schema).await?,
        };

        Ok(ResolvedVariable {
            name: schema.name.clone(),
            value,
            is_secret: schema.is_secret,
            source,
        })
    }

    /// Resolve personal-scoped variable
    async fn resolve_personal_variable(
        &self,
        schema: &VariableSchema,
    ) -> Result<(serde_json::Value, VariableSource)> {
        // Try in order: keychain -> environment -> default

        if schema.is_secret {
            // Check keychain first for secrets
            if let Ok(value) = self.get_from_keychain(&schema.name).await {
                return Ok((serde_json::Value::String(value), VariableSource::Keychain));
            }
        }

        // Check environment variables
        if let Ok(value) = env::var(&schema.name) {
            return Ok((
                serde_json::Value::String(value),
                VariableSource::Environment,
            ));
        }

        // Try 1Password if configured
        if schema.is_secret {
            if let Ok(value) = self.get_from_1password(&schema.name).await {
                return Ok((
                    serde_json::Value::String(value),
                    VariableSource::OnePassword,
                ));
            }
        }

        // Fall back to default
        if let Some(default) = &schema.default {
            return Ok((default.clone(), VariableSource::RssDefault));
        }

        Err(RelayError::VariableResolution(format!(
            "Variable '{}' not found in any source",
            schema.name
        )))
    }

    /// Resolve workspace-scoped variable
    async fn resolve_workspace_variable(
        &self,
        schema: &VariableSchema,
    ) -> Result<(serde_json::Value, VariableSource)> {
        let workspace_key = format!(
            "workspace_{}_{}",
            self.workspace_id.as_deref().unwrap_or("default"),
            schema.name
        );

        if schema.is_secret {
            if let Ok(value) = self.get_from_keychain(&workspace_key).await {
                return Ok((serde_json::Value::String(value), VariableSource::Keychain));
            }
        }

        // Fall back to personal scope
        self.resolve_personal_variable(schema).await
    }

    /// Resolve project-scoped variable (highest priority)
    async fn resolve_project_variable(
        &self,
        schema: &VariableSchema,
    ) -> Result<(serde_json::Value, VariableSource)> {
        let project_key = format!(
            "project_{}_{}",
            self.project_id.as_deref().unwrap_or("default"),
            schema.name
        );

        if schema.is_secret {
            if let Ok(value) = self.get_from_keychain(&project_key).await {
                return Ok((serde_json::Value::String(value), VariableSource::Keychain));
            }
        }

        // Fall back to workspace scope
        self.resolve_workspace_variable(schema).await
    }

    /// Get value from OS keychain
    async fn get_from_keychain(&self, key: &str) -> Result<String> {
        let entry = keyring::Entry::new("skill-cookbook-relay-vars", key)
            .map_err(|e| RelayError::Keychain(format!("Keychain access error: {}", e)))?;

        entry
            .get_password()
            .map_err(|e| RelayError::Keychain(format!("Failed to get keychain value: {}", e)))
    }

    /// Get value from 1Password CLI (if available)
    async fn get_from_1password(&self, reference: &str) -> Result<String> {
        // Check if op CLI is available
        let output = tokio::process::Command::new("op")
            .arg("--version")
            .output()
            .await;

        if output.is_err() {
            return Err(RelayError::VariableResolution(
                "1Password CLI not available".to_string(),
            ));
        }

        // Try to read from 1Password using reference format: op://vault/item/field
        let output = tokio::process::Command::new("op")
            .arg("read")
            .arg(reference)
            .output()
            .await
            .map_err(|e| {
                RelayError::VariableResolution(format!("Failed to execute op CLI: {}", e))
            })?;

        if !output.status.success() {
            return Err(RelayError::VariableResolution(format!(
                "1Password read failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let value = String::from_utf8(output.stdout)
            .map_err(|e| {
                RelayError::VariableResolution(format!("Invalid UTF-8 from op CLI: {}", e))
            })?
            .trim()
            .to_string();

        Ok(value)
    }

    /// Store a variable in the appropriate scope
    pub async fn store_variable(
        &self,
        name: &str,
        value: &str,
        scope: VariableScope,
        is_secret: bool,
    ) -> Result<()> {
        if !is_secret {
            return Err(RelayError::VariableResolution(
                "Non-secret variables should be stored in config files".to_string(),
            ));
        }

        let key = match scope {
            VariableScope::Personal => name.to_string(),
            VariableScope::Workspace => {
                format!(
                    "workspace_{}_{}",
                    self.workspace_id.as_deref().unwrap_or("default"),
                    name
                )
            }
            VariableScope::Project => {
                format!(
                    "project_{}_{}",
                    self.project_id.as_deref().unwrap_or("default"),
                    name
                )
            }
        };

        let entry = keyring::Entry::new("skill-cookbook-relay-vars", &key)
            .map_err(|e| RelayError::Keychain(format!("Keychain access error: {}", e)))?;

        entry
            .set_password(value)
            .map_err(|e| RelayError::Keychain(format!("Failed to store in keychain: {}", e)))?;

        tracing::info!(
            "Stored variable '{}' in keychain (scope: {:?})",
            name,
            scope
        );

        Ok(())
    }

    /// Delete a variable from storage
    pub async fn delete_variable(&self, name: &str, scope: VariableScope) -> Result<()> {
        let key = match scope {
            VariableScope::Personal => name.to_string(),
            VariableScope::Workspace => {
                format!(
                    "workspace_{}_{}",
                    self.workspace_id.as_deref().unwrap_or("default"),
                    name
                )
            }
            VariableScope::Project => {
                format!(
                    "project_{}_{}",
                    self.project_id.as_deref().unwrap_or("default"),
                    name
                )
            }
        };

        let entry = keyring::Entry::new("skill-cookbook-relay-vars", &key)
            .map_err(|e| RelayError::Keychain(format!("Keychain access error: {}", e)))?;

        entry
            .delete_password()
            .map_err(|e| RelayError::Keychain(format!("Failed to delete from keychain: {}", e)))?;

        tracing::info!(
            "Deleted variable '{}' from keychain (scope: {:?})",
            name,
            scope
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_from_env() {
        env::set_var("TEST_VAR", "test_value");

        let resolver = VariableResolver::new(None, None);
        let schema = VariableSchema {
            name: "TEST_VAR".to_string(),
            var_type: "string".to_string(),
            scope: VariableScope::Personal,
            is_secret: false,
            required: true,
            default: None,
            description: None,
        };

        let result = resolver.resolve_variable(&schema).await;
        assert!(result.is_ok());

        let resolved = result.unwrap();
        assert_eq!(resolved.name, "TEST_VAR");
        assert_eq!(
            resolved.value,
            serde_json::Value::String("test_value".to_string())
        );
        assert_eq!(resolved.source, VariableSource::Environment);

        env::remove_var("TEST_VAR");
    }

    #[tokio::test]
    async fn test_resolve_required_var_missing() {
        let resolver = VariableResolver::new(None, None);
        let schema = VariableSchema {
            name: "MISSING_VAR".to_string(),
            var_type: "string".to_string(),
            scope: VariableScope::Personal,
            is_secret: false,
            required: true,
            default: None,
            description: None,
        };
        let result = resolver.resolve_variable(&schema).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("MISSING_VAR"));
        }
    }

    #[tokio::test]
    async fn test_resolve_optional_var_missing() {
        let resolver = VariableResolver::new(None, None);
        let schema = VariableSchema {
            name: "OPTIONAL_VAR".to_string(),
            var_type: "string".to_string(),
            scope: VariableScope::Personal,
            is_secret: false,
            required: false,
            default: None,
            description: None,
        };
        let result = resolver.resolve_variable(&schema).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_with_default() {
        let resolver = VariableResolver::new(None, None);
        let schema = VariableSchema {
            name: "VAR_WITH_DEFAULT".to_string(),
            var_type: "string".to_string(),
            scope: VariableScope::Personal,
            is_secret: false,
            required: false,
            default: Some(serde_json::Value::String("default_value".to_string())),
            description: None,
        };
        let result = resolver.resolve_variable(&schema).await;
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(
            resolved.value,
            serde_json::Value::String("default_value".to_string())
        );
        assert_eq!(resolved.source, VariableSource::RssDefault);
    }

    #[tokio::test]
    async fn test_missing_required_variables() {
        let resolver = VariableResolver::new(None, None);
        let schemas = vec![
            VariableSchema {
                name: "VAR1".to_string(),
                var_type: "string".to_string(),
                scope: VariableScope::Personal,
                is_secret: false,
                required: true,
                default: None,
                description: None,
            },
            VariableSchema {
                name: "VAR2".to_string(),
                var_type: "string".to_string(),
                scope: VariableScope::Personal,
                is_secret: false,
                required: true,
                default: Some(serde_json::Value::String("default".to_string())),
                description: None,
            },
            VariableSchema {
                name: "VAR3".to_string(),
                var_type: "string".to_string(),
                scope: VariableScope::Personal,
                is_secret: false,
                required: false,
                default: None,
                description: None,
            },
        ];
        env::set_var("VAR1", "value1");
        let missing = resolver.missing_required_variables(&schemas).await;
        env::remove_var("VAR1");
        assert_eq!(missing.len(), 0);
    }
}
