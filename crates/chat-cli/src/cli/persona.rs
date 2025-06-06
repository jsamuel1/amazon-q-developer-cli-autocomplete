#![allow(dead_code)]

use std::borrow::Borrow;
use std::collections::{
    HashMap,
    HashSet,
};
use std::ffi::OsStr;
use std::hash::Hash;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

use crossterm::{
    queue,
    style,
};
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};
use tokio::fs::ReadDir;

pub type McpServerName = String;
pub type HookName = String;

pub(crate) enum PermissionEvalResult {
    Allow,
    Deny,
    Ask,
}

/// To be implemented by tools
/// The intended workflow here is to utilize to the visitor pattern
/// - [ToolPermissions] accepts a PermissionCandidate
/// - it then passes a reference of itself to [PermissionCandidate::eval]
/// - it is then expected to look through the permissions hashmap to conclude
pub(crate) trait PermissionCandidate {
    fn eval(&self, tool_permissions: &ToolPermissions) -> PermissionEvalResult;
}

#[derive(Debug, Serialize, Eq)]
pub(crate) enum PermissionSubject {
    All,
    ExactName(String),
}

impl PartialEq for PermissionSubject {
    fn eq(&self, other: &Self) -> bool {
        <Self as Borrow<str>>::borrow(self) == <Self as Borrow<str>>::borrow(other)
    }
}

impl Hash for PermissionSubject {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        <Self as Borrow<str>>::borrow(self).hash(state);
    }
}

impl Borrow<str> for PermissionSubject {
    fn borrow(&self) -> &str {
        match self {
            PermissionSubject::All => "*",
            PermissionSubject::ExactName(name) => name.as_str(),
        }
    }
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
pub(crate) struct Hook {
    trigger: Trigger,
    command: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Trigger {
    PerPrompt,
    ConversationStart,
}

#[derive(Debug, Serialize)]
pub(crate) enum DetailedListArgs {
    GlobSet(),
    Command(String),
}

/// Represents the permission level for a tool execution.
///
/// This enum defines how tools can be executed within the system, providing
/// granular control over tool access and security. Tools can be completely
/// allowed, completely denied, or have specific rules based on their arguments
/// or commands.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum ToolPermission {
    /// Can be executed without asking for permission
    AlwaysAllow,
    /// Cannot be executed
    Deny,
    /// A more nuanced way of specifying what gets permitted.
    /// The content of the vector are arguments / command with which the tool is run.
    /// Because the way they are interpreted is dependent on the tool, this is most expected to be
    /// used on native tools such as fs_read / fs_write (at least until further notice).
    /// For now, vectors contain String, or the arguments in their most primitive forms.
    /// This is because this field is overloaded, and it is best to leave any further
    /// deserialization to the individual tools that are receiving this config. This simplifies the
    /// deserialization process on a schema level at the cost of performance during a tool call.
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
pub(crate) struct ToolPermissions {
    #[serde(rename = "builtIn")]
    pub built_in: HashMap<PermissionSubject, ToolPermission>,
    #[serde(flatten)]
    pub custom: HashMap<PermissionSubject, HashMap<PermissionSubject, ToolPermission>>,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            built_in: {
                let mut perms = HashMap::<PermissionSubject, ToolPermission>::new();
                perms.insert(
                    PermissionSubject::ExactName("fs_read".to_string()),
                    ToolPermission::AlwaysAllow,
                );
                perms.insert(
                    PermissionSubject::ExactName("report_issue".to_string()),
                    ToolPermission::AlwaysAllow,
                );
                perms
            },
            custom: Default::default(),
        }
    }
}

impl ToolPermissions {
    pub fn evaluate(&self, candidate: &impl PermissionCandidate) -> PermissionEvalResult {
        candidate.eval(self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Context {
    files: Vec<PathBuf>,
    hooks: HashMap<HookName, Hook>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            files: {
                vec!["AmazonQ.md", "README.md", ".amazonq/rules/**/*.md"]
                    .into_iter()
                    .filter_map(|s| PathBuf::from_str(s).ok())
                    .collect::<Vec<_>>()
            },
            hooks: Default::default(),
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub(crate) enum McpServerList {
    #[default]
    All,
    List(Vec<McpServerName>),
}

impl<'de> Deserialize<'de> for McpServerList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::fmt;

        use serde::de::Visitor;

        struct ServerListVisitor;

        impl<'de> Visitor<'de> for ServerListVisitor {
            type Value = McpServerList;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut list = Vec::<McpServerName>::new();

                while let Ok(Some(value)) = seq.next_element::<McpServerName>() {
                    if value == "*" {
                        return Ok(McpServerList::All);
                    }
                    list.push(value);
                }

                Ok(McpServerList::List(list))
            }
        }

        deserializer.deserialize_seq(ServerListVisitor)
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonaConfig {
    mcp_servers: McpServerList,
    tool_perms: ToolPermissions,
    context: Context,
}

pub(crate) enum Persona {
    Local {
        path: PathBuf,
        name: String,
        config: PersonaConfig,
    },
    Global {
        name: String,
        config: PersonaConfig,
    },
}

impl Default for Persona {
    fn default() -> Self {
        Self::Global {
            name: "Default".to_string(),
            config: Default::default(),
        }
    }
}

impl Persona {
    pub async fn load(output: &mut impl Write) -> Vec<Self> {
        let mut local_personas = 'local: {
            let Ok(mut cwd) = std::env::current_dir() else {
                break 'local Vec::<Self>::new();
            };
            cwd.push(".amazonq/personas");
            let Ok(files) = tokio::fs::read_dir(cwd).await else {
                break 'local Vec::<Self>::new();
            };
            load_personas_from_entries(files, false).await
        };

        let mut global_personas = 'global: {
            let expanded_path = shellexpand::tilde("~/.aws/amazonq/personas");
            let global_path = PathBuf::from(expanded_path.as_ref() as &str);
            let Ok(files) = tokio::fs::read_dir(global_path).await else {
                break 'global Vec::<Self>::new();
            };
            load_personas_from_entries(files, true).await
        };

        let local_names = local_personas
            .iter()
            .filter_map(|p| {
                if let Persona::Local { name, .. } = p {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect::<HashSet<&str>>();

        global_personas.retain(|p| {
            if let Persona::Global { name, .. } = &p {
                let _ = queue!(
                    output,
                    style::SetForegroundColor(style::Color::Yellow),
                    style::Print("WARNING: "),
                    style::ResetColor,
                    style::Print("Persona conflict for "),
                    style::SetForegroundColor(style::Color::Green),
                    style::Print(name),
                    style::ResetColor,
                    style::Print(". Using workspace version.\n")
                );
                !local_names.contains(name.as_str())
            } else {
                false
            }
        });
        let _ = output.flush();

        local_personas.append(&mut global_personas);

        local_personas
    }
}

async fn load_personas_from_entries(mut files: ReadDir, is_global: bool) -> Vec<Persona> {
    let mut res = Vec::<Persona>::new();

    while let Ok(Some(file)) = files.next_entry().await {
        let file_path = &file.path();
        if file_path
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|s| s == "json")
        {
            let content = match tokio::fs::read(file_path).await {
                Ok(content) => content,
                Err(e) => {
                    let file_path = file_path.to_string_lossy();
                    tracing::error!("Error reading persona file {file_path}: {:?}", e);
                    continue;
                },
            };
            let config = match serde_json::from_slice::<PersonaConfig>(&content) {
                Ok(persona) => persona,
                Err(e) => {
                    let file_path = file_path.to_string_lossy();
                    tracing::error!("Error deserializing persona file {file_path}: {:?}", e);
                    continue;
                },
            };
            let name = file.file_name().to_str().unwrap_or("unknown_persona").to_string();
            if is_global {
                res.push(Persona::Global { name, config });
            } else {
                res.push(Persona::Local {
                    path: file.path(),
                    name,
                    config,
                });
            }
        }
    }

    res
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

    const MCP_SERVERS_LIST_ALL: &str = r#"["*"]"#;

    #[test]
    fn test_deserialize_mcp_server_list() {
        let list = serde_json::from_str::<McpServerList>(MCP_SERVERS_LIST_ALL);
        assert!(list.is_ok());
        let list = list.unwrap();
        assert!(matches!(list, McpServerList::All));
    }

    #[test]
    fn test_deserialize_persona_config() {
        let persona_config = serde_json::from_str::<PersonaConfig>(INPUT);
        assert!(persona_config.is_ok());
        let persona_config = persona_config.unwrap();
        assert!(matches!(persona_config.mcp_servers, McpServerList::List(_)));
        let McpServerList::List(servers) = persona_config.mcp_servers else {
            panic!("Server list should be a sequence in this test case");
        };
        let servers = &servers.iter().map(String::as_str).collect::<Vec<&str>>();
        assert!(servers.contains(&"fetch"));
        assert!(servers.contains(&"git"));

        let perms = &persona_config.tool_perms;
        assert!(perms.built_in.contains_key("fs_read"));
        assert!(perms.built_in.contains_key("use_aws"));
        assert!(perms.built_in.contains_key("execute_bash"));
        assert!(perms.custom.contains_key("git"));
        assert!(perms.custom.contains_key("fetch"));

        let context = &persona_config.context;
        assert!(context.files.len() == 1);
        assert!(context.hooks.contains_key("git-status"));
        assert!(context.hooks.contains_key("project-info"));
    }
}
