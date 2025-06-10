use std::collections::{
    HashMap,
    HashSet,
};
use std::io::Write;
use std::path::{
    Path,
    PathBuf,
};

use crossterm::{
    queue,
    style,
};
use serde::{
    Deserialize,
    Serialize,
};

use super::chat::tools::custom_tool::CustomToolConfig;
use crate::platform::Context;

// This is to mirror claude's config set up
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub mcp_servers: HashMap<String, CustomToolConfig>,
}

impl McpServerConfig {
    pub async fn load_config(output: &mut impl Write) -> eyre::Result<Self> {
        let mut cwd = std::env::current_dir()?;
        cwd.push(".amazonq/mcp.json");
        let expanded_path = shellexpand::tilde("~/.aws/amazonq/mcp.json");
        let global_path = PathBuf::from(expanded_path.as_ref() as &str);
        let global_buf = tokio::fs::read(global_path).await.ok();
        let local_buf = tokio::fs::read(cwd).await.ok();
        let conf = match (global_buf, local_buf) {
            (Some(global_buf), Some(local_buf)) => {
                let mut global_conf = Self::from_slice(&global_buf, output, "global")?;
                let local_conf = Self::from_slice(&local_buf, output, "local")?;
                for (server_name, config) in local_conf.mcp_servers {
                    if global_conf.mcp_servers.insert(server_name.clone(), config).is_some() {
                        queue!(
                            output,
                            style::SetForegroundColor(style::Color::Yellow),
                            style::Print("WARNING: "),
                            style::ResetColor,
                            style::Print("MCP config conflict for "),
                            style::SetForegroundColor(style::Color::Green),
                            style::Print(server_name),
                            style::ResetColor,
                            style::Print(". Using workspace version.\n")
                        )?;
                    }
                }
                global_conf
            },
            (None, Some(local_buf)) => Self::from_slice(&local_buf, output, "local")?,
            (Some(global_buf), None) => Self::from_slice(&global_buf, output, "global")?,
            _ => Default::default(),
        };
        output.flush()?;
        Ok(conf)
    }

    pub async fn load_from_file(ctx: &Context, path: impl AsRef<Path>) -> eyre::Result<Self> {
        let contents = ctx.fs().read_to_string(path.as_ref()).await?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub async fn save_to_file(&self, ctx: &Context, path: impl AsRef<Path>) -> eyre::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        ctx.fs().write(path.as_ref(), json).await?;
        Ok(())
    }

    fn from_slice(slice: &[u8], output: &mut impl Write, location: &str) -> eyre::Result<McpServerConfig> {
        match serde_json::from_slice::<Self>(slice) {
            Ok(config) => Ok(config),
            Err(e) => {
                queue!(
                    output,
                    style::SetForegroundColor(style::Color::Yellow),
                    style::Print("WARNING: "),
                    style::ResetColor,
                    style::Print(format!("Error reading {location} mcp config: {e}\n")),
                    style::Print("Please check to make sure config is correct. Discarding.\n"),
                )?;
                Ok(McpServerConfig::default())
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub allowed_tools: HashSet<String>,
    #[serde(default)]
    pub file_hooks: Vec<String>,
    #[serde(default)]
    pub start_hooks: Vec<String>,
    #[serde(default)]
    pub prompt_hooks: Vec<String>,
    #[serde(default)]
    pub tools_settings: HashMap<String, serde_json::Value>,
}

pub enum PermissionEvalResult {
    Allow,
    Ask,
    Deny,
}

impl Agent {
    pub fn eval(&self, candidate: &impl PermissionCandidate) -> PermissionEvalResult {
        candidate.eval(self)
    }
}

/// To be implemented by tools
/// The intended workflow here is to utilize to the visitor pattern
/// - [Agent] accepts a PermissionCandidate
/// - it then passes a reference of itself to [PermissionCandidate::eval]
/// - it is then expected to look through the permissions hashmap to conclude
pub trait PermissionCandidate {
    fn eval(&self, agent: &Agent) -> PermissionEvalResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str = r#"
            {
              "name": "my_developer_agent",
              "description": "My developer agent is used for small development tasks like solving open issues.",
              "prompt": "You are a principal developer who uses multiple agents to accomplish difficult engineering tasks",
              "servers": {
                "fetch": { "command": "fetch3.1", "args": {} },
                "git": { "command": "git-mcp", "args": {} }
              },
              "tools": [                                    
                "@git",                                     # can be either the full mcp-server
                "@git/git_status",                          # or just one tool from an MCP server (no validation done on whether the server has that tool)
                "\#developer",
                "fs_read"
              ],
              "allowedTools": [                             # tools without permissions
                "fs_read",                                  # to add further granularity, it must first be in allowed tools
                "@fetch",
                "@git/git_status"
              ],
              "includedFiles": [                            # same as context files
                "~/my-genai-prompts/unittest.md"
              ],
              "createHooks": [                              # same as conversation-start-hooks
                "pwd && tree"
              ],
              "promptHooks": [                              # same as per prompt hooks
                "git status"
              ],
              "toolsSettings": {                            # per-tool settings
                "fs_write": { "allowedPaths": ["~/**"] },
                "@git/git_status": { "git_user": "$GIT_USER" }
              }
            }
        "#;

    #[test]
    fn test_deser() {
        let agent = serde_json::from_str::<Agent>(INPUT).expect("Agent config deserialization failed");
        assert!(agent.name == "my_developer_agent");
        assert!(agent.servers.contains_key("fetch"));
        assert!(agent.servers.contains_key("git"));
    }
}
