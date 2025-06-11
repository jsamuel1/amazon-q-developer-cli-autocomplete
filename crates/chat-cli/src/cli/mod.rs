mod chat;
mod debug;
mod diagnostics;
mod feed;
mod issue;
mod mcp;
mod settings;
mod user;

use std::fmt::Display;
use std::io::{
    Write as _,
    stdout,
};
use std::process::ExitCode;

use anstream::println;
pub use chat::ConversationState;
use clap::{
    ArgAction,
    CommandFactory,
    Parser,
    ValueEnum,
};
use crossterm::style::Stylize;
use eyre::{
    Result,
    bail,
};
use feed::Feed;
use serde::Serialize;
use tracing::{
    Level,
    debug,
};

use crate::cli::chat::ChatArgs;
use crate::cli::mcp::McpSubcommand;
use crate::cli::user::{
    LoginArgs,
    WhoamiArgs,
};
use crate::logging::{
    LogArgs,
    initialize_logging,
};
use crate::util::directories::logs_dir;
use crate::util::{
    CLI_BINARY_NAME,
    CliContext,
    GOV_REGIONS,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Outputs the results as markdown
    #[default]
    Plain,
    /// Outputs the results as JSON
    Json,
    /// Outputs the results as pretty print JSON
    JsonPretty,
}

impl OutputFormat {
    pub fn print<T, TFn, J, JFn>(&self, text_fn: TFn, json_fn: JFn)
    where
        T: std::fmt::Display,
        TFn: FnOnce() -> T,
        J: Serialize,
        JFn: FnOnce() -> J,
    {
        match self {
            OutputFormat::Plain => println!("{}", text_fn()),
            OutputFormat::Json => println!("{}", serde_json::to_string(&json_fn()).unwrap()),
            OutputFormat::JsonPretty => println!("{}", serde_json::to_string_pretty(&json_fn()).unwrap()),
        }
    }
}

/// The Amazon Q CLI
#[deny(missing_docs)]
#[derive(Debug, PartialEq, clap::Subcommand)]
pub enum RootSubcommand {
    /// AI assistant in your terminal
    Chat(ChatArgs),
    /// Log in to Amazon Q
    Login(LoginArgs),
    /// Log out of Amazon Q
    Logout,
    /// Print info about the current login session
    Whoami(WhoamiArgs),
    /// Show the profile associated with this idc user
    Profile,
    /// Customize appearance & behavior
    #[command(alias("setting"))]
    Settings(settings::SettingsArgs),
    /// Run diagnostic tests
    #[command(alias("diagnostics"))]
    Diagnostic(diagnostics::DiagnosticArgs),
    /// Create a new Github issue
    Issue(issue::IssueArgs),
    /// Version
    #[command(hide = true)]
    Version {
        /// Show the changelog (use --changelog=all for all versions, or --changelog=x.x.x for a
        /// specific version)
        #[arg(long, num_args = 0..=1, default_missing_value = "")]
        changelog: Option<String>,
    },
    /// Model Context Protocol (MCP)
    #[command(subcommand)]
    Mcp(McpSubcommand),
}

impl RootSubcommand {
    /// Whether the command should have an associated telemetry event.
    ///
    /// Emitting telemetry takes a long time so the answer is usually no.
    pub fn valid_for_telemetry(&self) -> bool {
        matches!(self, Self::Chat(_) | Self::Login(_) | Self::Profile | Self::Issue(_))
    }

    pub fn requires_auth(&self) -> bool {
        matches!(self, Self::Chat(_) | Self::Profile)
    }
}

impl Default for RootSubcommand {
    fn default() -> Self {
        Self::Chat(ChatArgs::default())
    }
}

impl Display for RootSubcommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Chat(_) => "chat",
            Self::Login(_) => "login",
            Self::Logout => "logout",
            Self::Whoami(_) => "whoami",
            Self::Profile => "profile",
            Self::Settings(_) => "settings",
            Self::Diagnostic(_) => "diagnostic",
            Self::Issue(_) => "issue",
            Self::Version { .. } => "version",
            Self::Mcp(_) => "mcp",
        };

        write!(f, "{name}")
    }
}

#[derive(Debug, Parser, PartialEq, Default)]
#[command(version, about, name = crate::util::CHAT_BINARY_NAME)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<RootSubcommand>,
    /// Increase logging verbosity
    #[arg(long, short = 'v', action = ArgAction::Count, global = true)]
    pub verbose: u8,
    /// Print help for all subcommands
    #[arg(long)]
    help_all: bool,
}

impl Cli {
    pub async fn execute(self) -> Result<ExitCode> {
        let subcommand = self.subcommand.unwrap_or_default();

        // Initialize our logger and keep around the guard so logging can perform as expected.
        let _log_guard = initialize_logging(LogArgs {
            log_level: match self.verbose > 0 {
                true => Some(
                    match self.verbose {
                        1 => Level::WARN,
                        2 => Level::INFO,
                        3 => Level::DEBUG,
                        _ => Level::TRACE,
                    }
                    .to_string(),
                ),
                false => None,
            },
            log_to_stdout: std::env::var_os("Q_LOG_STDOUT").is_some() || self.verbose > 0,
            log_file_path: match subcommand {
                RootSubcommand::Chat { .. } => Some(logs_dir().expect("home dir must be set").join("qchat.log")),
                _ => None,
            },
            delete_old_log_file: false,
        });

        // Check for region support.
        if let Ok(region) = std::env::var("AWS_REGION") {
            if GOV_REGIONS.contains(&region.as_str()) {
                bail!("AWS GovCloud ({region}) is not supported.")
            }
        }

        debug!(command =? std::env::args().collect::<Vec<_>>(), "Command being ran");

        let env = crate::platform::Env::new();
        let mut database = crate::database::Database::new().await?;
        let telemetry = crate::telemetry::TelemetryThread::new(&env, &mut database).await?;

        // Check for auth on subcommands that require it.
        if subcommand.requires_auth() && !crate::auth::is_logged_in(&mut database).await {
            bail!(
                "You are not logged in, please log in with {}",
                format!("{CLI_BINARY_NAME} login").bold()
            );
        }

        // Send executed telemetry.
        if subcommand.valid_for_telemetry() {
            telemetry.send_cli_subcommand_executed(&subcommand).ok();
        }

        let cli_context = CliContext::new();

        let result = match subcommand {
            RootSubcommand::Diagnostic(args) => args.execute().await,
            RootSubcommand::Login(args) => args.execute(&mut database, &telemetry).await,
            RootSubcommand::Logout => user::logout(&mut database).await,
            RootSubcommand::Whoami(args) => args.execute(&mut database).await,
            RootSubcommand::Profile => user::profile(&mut database, &telemetry).await,
            RootSubcommand::Settings(settings_args) => settings_args.execute(&mut database, &cli_context).await,
            RootSubcommand::Issue(args) => args.execute().await,
            RootSubcommand::Version { changelog } => Self::print_version(changelog),
            RootSubcommand::Chat(args) => args.execute(&mut database, &telemetry).await,
            RootSubcommand::Mcp(args) => args.execute().await,
        };

        let telemetry_result = telemetry.finish().await;

        let exit_code = result?;
        telemetry_result?;
        Ok(exit_code)
    }

    fn print_changelog_entry(entry: &feed::Entry) -> Result<()> {
        println!("Version {} ({})", entry.version, entry.date);

        if entry.changes.is_empty() {
            println!("  No changes recorded for this version.");
        } else {
            for change in &entry.changes {
                let type_label = match change.change_type.as_str() {
                    "added" => "Added",
                    "fixed" => "Fixed",
                    "changed" => "Changed",
                    other => other,
                };

                println!("  - {}: {}", type_label, change.description);
            }
        }

        println!();
        Ok(())
    }

    fn print_version(changelog: Option<String>) -> Result<ExitCode> {
        // If no changelog is requested, display normal version information
        if changelog.is_none() {
            let _ = writeln!(stdout(), "{}", Self::command().render_version());
            return Ok(ExitCode::SUCCESS);
        }

        let changelog_value = changelog.unwrap_or_default();
        let feed = Feed::load();

        // Display changelog for all versions
        if changelog_value == "all" {
            let entries = feed.get_all_changelogs();
            if entries.is_empty() {
                println!("No changelog information available.");
            } else {
                println!("Changelog for all versions:");
                for entry in entries {
                    Self::print_changelog_entry(&entry)?;
                }
            }
            return Ok(ExitCode::SUCCESS);
        }

        // Display changelog for a specific version (--changelog=x.x.x)
        if !changelog_value.is_empty() {
            match feed.get_version_changelog(&changelog_value) {
                Some(entry) => {
                    println!("Changelog for version {}:", changelog_value);
                    Self::print_changelog_entry(&entry)?;
                    return Ok(ExitCode::SUCCESS);
                },
                None => {
                    println!("No changelog information available for version {}.", changelog_value);
                    return Ok(ExitCode::SUCCESS);
                },
            }
        }

        // Display changelog for the current version (--changelog only)
        let current_version = env!("CARGO_PKG_VERSION");
        match feed.get_version_changelog(current_version) {
            Some(entry) => {
                println!("Changelog for version {}:", current_version);
                Self::print_changelog_entry(&entry)?;
            },
            None => {
                println!("No changelog information available for version {}.", current_version);
            },
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::CHAT_BINARY_NAME;
    use crate::util::test::assert_parse;

    #[test]
    fn debug_assert() {
        Cli::command().debug_assert();
    }

    /// Test flag parsing for the top level [Cli]
    #[test]
    fn test_flags() {
        assert_eq!(Cli::parse_from([CHAT_BINARY_NAME, "-v"]), Cli {
            subcommand: None,
            verbose: 1,
            help_all: false,
        });

        assert_eq!(Cli::parse_from([CHAT_BINARY_NAME, "-vvv"]), Cli {
            subcommand: None,
            verbose: 3,
            help_all: false,
        });

        assert_eq!(Cli::parse_from([CHAT_BINARY_NAME, "--help-all"]), Cli {
            subcommand: None,
            verbose: 0,
            help_all: true,
        });

        assert_eq!(Cli::parse_from([CHAT_BINARY_NAME, "chat", "-vv"]), Cli {
            subcommand: Some(RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: None,
            })),
            verbose: 2,
            help_all: false,
        });
    }

    #[test]
    fn test_version_changelog() {
        assert_parse!(["version", "--changelog"], RootSubcommand::Version {
            changelog: Some("".to_string()),
        });
    }

    #[test]
    fn test_version_changelog_all() {
        assert_parse!(["version", "--changelog=all"], RootSubcommand::Version {
            changelog: Some("all".to_string()),
        });
    }

    #[test]
    fn test_version_changelog_specific() {
        assert_parse!(["version", "--changelog=1.8.0"], RootSubcommand::Version {
            changelog: Some("1.8.0".to_string()),
        });
    }

    #[test]
    fn test_chat_with_context_profile() {
        assert_parse!(
            ["chat", "--profile", "my-profile"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: Some("my-profile".to_string()),
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_context_profile_and_input() {
        assert_parse!(
            ["chat", "--profile", "my-profile", "Hello"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: Some("Hello".to_string()),
                profile: Some("my-profile".to_string()),
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_context_profile_and_accept_all() {
        assert_parse!(
            ["chat", "--profile", "my-profile", "--accept-all"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: true,
                no_interactive: false,
                resume: false,
                input: None,
                profile: Some("my-profile".to_string()),
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_no_interactive_and_resume() {
        assert_parse!(
            ["chat", "--no-interactive", "--resume"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: true,
                resume: true,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: None,
            })
        );
        assert_parse!(
            ["chat", "--no-interactive", "-r"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: true,
                resume: true,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_tool_trust_all() {
        assert_parse!(
            ["chat", "--trust-all-tools"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: true,
                trust_tools: None,
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_tool_trust_none() {
        assert_parse!(
            ["chat", "--trust-tools="],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: Some(vec!["".to_string()]),
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_tool_trust_some() {
        assert_parse!(
            ["chat", "--trust-tools=fs_read,fs_write"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: Some(vec!["fs_read".to_string(), "fs_write".to_string()]),
                mcp_config_paths: None,
            })
        );
    }

    #[test]
    fn test_chat_with_mcp_config_paths_single() {
        assert_parse!(
            ["chat", "--mcp-config-paths=/path/to/config.json"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: Some(vec!["/path/to/config.json".to_string()]),
            })
        );
    }

    #[test]
    fn test_chat_with_mcp_config_paths_multiple() {
        // Use the platform-appropriate path separator
        #[cfg(windows)]
        let paths_arg = "--mcp-config-paths=/path/to/config1.json;/path/to/config2.json";
        #[cfg(not(windows))]
        let paths_arg = "--mcp-config-paths=/path/to/config1.json:/path/to/config2.json";
        
        assert_parse!(
            ["chat", paths_arg],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: None,
                profile: None,
                model: None,
                trust_all_tools: false,
                trust_tools: None,
                mcp_config_paths: Some(vec![
                    "/path/to/config1.json".to_string(),
                    "/path/to/config2.json".to_string()
                ]),
            })
        );
    }

    #[test]
    fn test_chat_with_mcp_config_paths_and_other_args() {
        assert_parse!(
            ["chat", "--mcp-config-paths=/path/to/config.json", "--trust-all-tools", "Hello"],
            RootSubcommand::Chat(ChatArgs {
                accept_all: false,
                no_interactive: false,
                resume: false,
                input: Some("Hello".to_string()),
                profile: None,
                model: None,
                trust_all_tools: true,
                trust_tools: None,
                mcp_config_paths: Some(vec!["/path/to/config.json".to_string()]),
            })
        );
    }
}
