mod github;
mod graphql;
mod markdown;
mod package;

use crate::markdown::ast::NEXT_OR_LATEST;
use crate::markdown::ast::UNRELEASED_HEADING;
use chrono::prelude::*;
use clap::{AppSettings, Parser, Subcommand};
use github::github_info::GitHubInfo;
use markdown::ast::Node;
use markdown::tokens::MarkdownToken;
use package::SemVer;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;

/// Make CHANGELOG.md changes easier
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Cli {
    /// The current working directory
    #[clap(long, default_value = ".", global = true)]
    pwd: String,

    /// The changelog filename
    #[clap(short, long, default_value = "CHANGELOG.md", global = true)]
    filename: String,

    /// The subcommand to run
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Add a new entry to the changelog in the "Added" section
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Add {
        /// A link to the commit, pr, issue, ...
        #[clap(conflicts_with = "message")]
        link: Option<String>,

        /// A manual message you want to add
        #[clap(short, long, conflicts_with = "link")]
        message: Option<String>,

        /// The section name to add the entry to
        #[clap(hide = true, default_value = "Added")]
        name: String,
    },

    /// Add a new entry to the changelog in the "Fixed" section
    Fix {
        /// A link to the commit, pr, issue, ...
        #[clap(conflicts_with = "message")]
        link: Option<String>,

        /// A manual message you want to add
        #[clap(short, long, conflicts_with = "link")]
        message: Option<String>,

        /// The section name to add the entry to
        #[clap(hide = true, default_value = "Fixed")]
        name: String,
    },

    /// Add a new entry to the changelog in the "Changed" section
    Change {
        /// A link to the commit, pr, issue, ...
        #[clap(conflicts_with = "message")]
        link: Option<String>,

        /// A manual message you want to add
        #[clap(short, long, conflicts_with = "link")]
        message: Option<String>,

        /// The section name to add the entry to
        #[clap(hide = true, default_value = "Changed")]
        name: String,
    },

    /// Add a new entry to the changelog in the "Deprecated" section
    Deprecate {
        /// A link to the commit, pr, issue, ...
        #[clap(conflicts_with = "message")]
        link: Option<String>,

        /// A manual message you want to add
        #[clap(short, long, conflicts_with = "link")]
        message: Option<String>,

        /// The section name to add the entry to
        #[clap(hide = true, default_value = "Deprecated")]
        name: String,
    },

    /// Add a new entry to the changelog in the "Removed" section
    Remove {
        /// A link to the commit, pr, issue, ...
        #[clap(conflicts_with = "message")]
        link: Option<String>,

        /// A manual message you want to add
        #[clap(short, long, conflicts_with = "link")]
        message: Option<String>,

        /// The section name to add the entry to
        #[clap(hide = true, default_value = "Removed")]
        name: String,
    },

    /// Release a new version
    Release {
        /// The version of the release, which can be one of: "major", "minor", "patch", "infer"
        /// (infer from current package.json version) or an explicit version number like "1.2.3"
        #[clap(default_value = "infer")]
        version: SemVer,
    },

    /// Get the release notes of a specific version (or unreleased)
    Notes {
        /// The version you want to get the notes from. Should be a valid semver version or one of
        /// "unreleased", "latest" or "next_or_latest".
        #[clap(default_value = NEXT_OR_LATEST)]
        version: String,
    },

    /// Get a list of all versions
    List {
        /// Amount of versions to show
        #[clap(short, long, default_value = "10")]
        amount: Amount,

        /// Shorthand for "--amount all"
        #[clap(long, conflicts_with = "amount")]
        all: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum Amount {
    All,
    Value(usize),
}

impl FromStr for Amount {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Amount::All),
            _ => Ok(Amount::Value(
                s.parse::<usize>().map_err(|_| "Invalid amount")?,
            )),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let pwd = std::fs::canonicalize(&args.pwd).expect("File path doesn't seem to exist");
    let file_path = pwd.join(&args.filename);

    // Raw changelog contents
    let contents = std::fs::read_to_string(&file_path)?;
    let mut root: Node = contents.parse()?;

    match &args.command {
        Commands::Add {
            link,
            message,
            name,
        }
        | Commands::Fix {
            link,
            message,
            name,
        }
        | Commands::Change {
            link,
            message,
            name,
        }
        | Commands::Remove {
            link,
            message,
            name,
        }
        | Commands::Deprecate {
            link,
            message,
            name,
        } => {
            if let Some(link) = link {
                let data: GitHubInfo = link.parse()?;
                root.add_list_item_to_section(name, data.to_string());
            } else if let Some(message) = message {
                root.add_list_item_to_section(name, message.to_string());
            }

            std::fs::write(file_path, root.to_string() + "\n")?;
        }
        Commands::Notes { version } => {
            if let Some(node) = root.get_contents_of_section(version) {
                print!("{}", node);
            } else {
                eprintln!("Couldn't find notes for version: {}", version);
            }
        }
        Commands::Release { version } => {
            let date = Local::now().format("%Y-%m-%d");

            if let Some(unreleased) = root.find_node_mut(&|node| {
                if let Some(MarkdownToken::H2(name)) = &node.data {
                    name == UNRELEASED_HEADING
                } else {
                    false
                }
            }) {
                // Convert to the new version
                unreleased.rename(&format!("[{}] - {}", version, date));

                // Insert new [Unreleased] section at the top
                let mut new_unreleased =
                    Node::from_token(MarkdownToken::H2(UNRELEASED_HEADING.to_string()));
                let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                let li = Node::from_token(MarkdownToken::ListItem("Nothing yet!".to_string()));

                ul.add_child(li);
                new_unreleased.add_child(ul);

                root.children
                    .get_mut(0)
                    .expect("Couldn't find main heading, is your CHANGELOG.md formatted correctly?")
                    .add_child_at(2, new_unreleased);

                // Update references at the bottom
                let c = root.clone();
                let old_version = c
                    .find_latest_version()
                    .expect("Couldn't find latest version");

                if let Some(unreleased_reference) = root.find_node_mut(&|node| {
                    if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                        name.eq_ignore_ascii_case("unreleased")
                    } else {
                        false
                    }
                }) {
                    if let Some(MarkdownToken::Reference(name, link)) = &unreleased_reference.data {
                        let (updated_link, new_link) = (
                            link.clone().replace(old_version, &version.to_string()),
                            link.clone().replace("HEAD", &format!("v{}", version)),
                        );

                        // Update unreleased_reference
                        unreleased_reference.data =
                            Some(MarkdownToken::Reference(name.to_string(), updated_link));

                        // Insert new version reference
                        let new_version_reference = Node::from_token(MarkdownToken::Reference(
                            version.to_string(),
                            new_link,
                        ));

                        match root.children.iter().position(|node| {
                            if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                                name.eq_ignore_ascii_case("unreleased")
                            } else {
                                false
                            }
                        }) {
                            Some(idx) => {
                                root.add_child_at(idx + 1, new_version_reference);
                            }
                            None => {
                                root.add_child(new_version_reference);
                            }
                        }
                    }
                }
            }

            std::fs::write(file_path, root.to_string() + "\n")?;
        }
        Commands::List { amount, all } => {
            let references = root
                .filter_nodes(&|node| matches!(&node.data, Some(MarkdownToken::Reference(_, _))))
                .iter()
                .filter_map(|node| node.data.as_ref())
                .take(match all {
                    true => std::usize::MAX,
                    false => match *amount {
                        Amount::All => std::usize::MAX,
                        Amount::Value(x) => x,
                    },
                })
                .map(|token| match token {
                    MarkdownToken::Reference(name, link) => format!("- {:15} {}", name, link),
                    _ => panic!("Expected a reference"),
                })
                .collect::<Vec<_>>()
                .join("\n");

            println!("{}", references);
        }
    };

    Ok(())
}
