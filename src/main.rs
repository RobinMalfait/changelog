mod github;
mod graphql;
mod markdown;
mod package;

use crate::markdown::ast::NEXT_OR_LATEST;
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
        Commands::Release { version: _ } => {
            println!("{:#?}", &args);
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
