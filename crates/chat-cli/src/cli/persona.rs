use std::collections::HashMap;
use std::path::PathBuf;

use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};

pub type McpServerName = String;
pub type HookName = String;

#[derive(Debug, Serialize, PartialEq, Eq, Hash)]
pub enum PermissionSubject {
    All,
    ExactName(String),
}

impl<'de> Deserialize<'de> for PermissionSubject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "*" {
            Ok(PermissionSubject::All)
        } else {
            Ok(PermissionSubject::ExactName(s))
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hook {
    trigger: Trigger,
    command: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    PerPrompt,
    ConversationStart,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ToolPermission {
    AlwaysAllow,
    Deny,
    DetailedList {
        #[serde(default)]
        always_allow: Vec<String>,
        #[serde(default)]
        deny: Vec<String>,
    },
}

impl<'de> Deserialize<'de> for ToolPermission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::fmt;

        use serde::de::{
            self,
            MapAccess,
            Visitor,
        };

        struct ToolPermissionVisitor;

        impl<'de> Visitor<'de> for ToolPermissionVisitor {
            type Value = ToolPermission;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "alwaysAllow" => Ok(ToolPermission::AlwaysAllow),
                    "deny" => Ok(ToolPermission::Deny),
                    _ => Err(de::Error::unknown_variant(value, &["alwaysAllow", "deny"])),
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut always_allow = Vec::new();
                let mut deny = Vec::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "alwaysAllow" => {
                            always_allow = map.next_value()?;
                        },
                        "deny" => {
                            deny = map.next_value()?;
                        },
                        _ => {
                            return Err(de::Error::unknown_field(&key, &["alwaysAllow", "deny"]));
                        },
                    }
                }

                Ok(ToolPermission::DetailedList { always_allow, deny })
            }
        }

        deserializer.deserialize_any(ToolPermissionVisitor)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissions {
    #[serde(rename = "builtIn")]
    built_in: HashMap<String, ToolPermission>,
    #[serde(flatten)]
    custom: HashMap<String, HashMap<String, ToolPermission>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    files: Vec<PathBuf>,
    hooks: HashMap<HookName, Hook>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    mcp_servers: Vec<McpServerName>,
    tool_perms: ToolPermissions,
    context: Context,
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str = r#"{
      "mcpServers": [                 
        "fetch",
        "git"
      ],
      "toolPerms": {                  
        "builtIn": {                  
          "fs_read": "alwaysAllow",   
          "use_aws": {
            "alwaysAllow": [
            ]
          },
          "fs_write": {
            "alwaysAllow": [          
              ".",
              "/var/www/**"           
            ],
            "deny": [                 
              "/etc"
            ]
          },
          "execute_bash": {
            "alwaysAllow": [
              "npm"                   
            ],
            "deny": [                 
              "curl"
            ]
          }
        },
        "git": {                      
          "git_status": "alwaysAllow",
          "git_commit": "deny"        
        },
        "fetch": {
          "*": "alwaysAllow"          
        }
      },
      "context": {
        "files": [
          "~/my-genai-prompts/unittest.md"
        ],
        "hooks": {
          "git-status": {
            "trigger": "per_prompt",
            "command": "git status"
          },
          "project-info": {
            "trigger": "conversation_start",
            "command": "pwd && tree"
          }
        }
      }
    }"#;

    #[test]
    fn test_deserialize() {
        let persona = serde_json::from_str::<Persona>(INPUT);
        assert!(persona.is_ok());
    }
}
