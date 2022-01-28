mod changelog;
mod git;
mod github;
mod graphql;
mod markdown;
mod npm;
mod output;
mod package;
mod rich_edit;

use crate::changelog::{Amount, Changelog};
use crate::git::Git;
use crate::github::github_info::GitHubInfo;
use crate::markdown::ast::Node;
use crate::markdown::tokens::MarkdownToken;
use crate::npm::Npm;
use crate::output::output;
use crate::output::output_indented;
use crate::package::PackageJSON;
use crate::package::SemVer;
use crate::rich_edit::rich_edit;
use clap::{AppSettings, Parser, Subcommand};
use color_eyre::eyre::Result;
use colored::*;
use dialoguer::{MultiSelect, Select};
use std::fmt::Debug;

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
    /// Initialize a new CHANGELOG.md file, if it doesn't exist yet
    Init,

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

        /// The name of the package (useful in monorepos)
        #[clap(short, long)]
        scope: Option<String>,

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,
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

        /// The name of the package (useful in monorepos)
        #[clap(short, long)]
        scope: Option<String>,

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,
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

        /// The name of the package (useful in monorepos)
        #[clap(short, long)]
        scope: Option<String>,

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,
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
        /// The name of the package (useful in monorepos)
        #[clap(short, long)]
        scope: Option<String>,

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,
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

        /// The name of the package (useful in monorepos)
        #[clap(short, long)]
        scope: Option<String>,

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,
    },

    /// Release a new version
    Release {
        /// The version of the release, which can be one of: "major", "minor", "patch", "infer"
        /// (infer from current package.json version) or an explicit version number like "1.2.3"
        #[clap(default_value = "infer")]
        version: SemVer,

        /// Whether or not to run `npm version <version>` (which in turn updates package.json and
        /// creates a new git tag)
        #[clap(long)]
        with_npm: bool,

        /// The name of the package (useful in monorepos)
        #[clap(short, long)]
        scope: Option<String>,
    },

    /// Get the release notes of a specific version (or unreleased)
    Notes {
        /// The version you want to get the notes from. Should be a valid semver version or one of
        /// "unreleased" or "latest".
        version: Option<String>,
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

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();

    let mut changelog = Changelog::new(&args.pwd, &args.filename)?;

    let pkg = PackageJSON::from_current_directory()?;

    match &args.command {
        Commands::Init => changelog.init(),
        Commands::Add {
            link,
            message,
            name,
            scope,
            commit,
        }
        | Commands::Fix {
            link,
            message,
            name,
            scope,
            commit,
        }
        | Commands::Change {
            link,
            message,
            name,
            scope,
            commit,
        }
        | Commands::Remove {
            link,
            message,
            name,
            scope,
            commit,
        }
        | Commands::Deprecate {
            link,
            message,
            name,
            scope,
            commit,
        } => {
            let scope = if !pkg.is_monorepo() {
                None
            } else if scope.is_none() {
                let options = pkg.packages()?;
                let idx = Select::new()
                    .with_prompt("Select a package to add the entry to:")
                    .items(&options)
                    .clear(true)
                    .default(0)
                    .interact()?;

                Some(options[idx].clone())
            } else {
                Some(scope.as_ref().unwrap().clone())
            };

            changelog.set_scope(scope.clone());
            changelog.parse_contents()?;

            let messages = if let Some(message) = message {
                changelog.add_list_item_to_section(name, message.to_string());
                vec![message.to_string()]
            } else if let Some(link) = link {
                let data: GitHubInfo = link.parse().unwrap();
                changelog.add_list_item_to_section(name, data.to_string());
                vec![data.to_string()]
            } else {
                let preface = &format!(
                    include_str!("./fixtures/add_entry.txt"),
                    name.to_lowercase()
                );

                let data = match rich_edit(Some(preface)) {
                    Some(data) => {
                        let data = data.trim();
                        let data: Vec<String> = data
                            .lines()
                            .into_iter()
                            .map(|line| line.trim())
                            .filter(|line| !line.is_empty())
                            .filter(|line| !line.starts_with('#'))
                            .map(|line| line.to_string())
                            .collect();

                        for line in &data {
                            changelog.add_list_item_to_section(name, line.to_string());
                        }

                        if data.is_empty() {
                            None
                        } else {
                            Some(data)
                        }
                    }
                    None => None,
                };

                match data {
                    Some(data) => data,
                    None => {
                        output(format!(
                            "No {}, {} or {} provided, run `{}` for more info",
                            "<LINK>".blue().bold(),
                            "<COMMIT HASH>".blue().bold(),
                            "--message".blue().bold(),
                            format!(
                                "changelog {} --help",
                                match &args.command {
                                    Commands::Add { .. } => "add",
                                    Commands::Fix { .. } => "fix",
                                    Commands::Change { .. } => "change",
                                    Commands::Remove { .. } => "remove",
                                    Commands::Deprecate { .. } => "deprecate",
                                    _ => unreachable!(),
                                }
                            )
                            .blue()
                            .bold()
                        ));
                        std::process::exit(1);
                    }
                }
            };

            output(format!(
                "Added a new entry to the {} section{}:",
                name.blue().bold(),
                match &scope {
                    Some(scope) => format!("{}", format!(" ({})", &scope).white().dimmed()),
                    None => "".to_string(),
                }
            ));

            if let Some(node) = changelog.get_contents_of_section(&None) {
                let mut text = node.to_string();

                for message in messages {
                    text = text.replace(
                        &format!("- {}", message),
                        &format!("- {}", message.green().bold()),
                    );
                }

                output_indented(text);
                eprintln!()
            }

            changelog.persist()?;

            if *commit {
                // Commit the CHANGELOG.md file
                Git::new(Some(&args.pwd))?
                    .add(changelog.file_path_str())?
                    .commit("update changelog")?;
            }

            Ok(())
        }
        Commands::Notes { version } => changelog.parse_contents()?.notes(version),
        Commands::Release {
            version,
            with_npm,
            scope,
        } => {
            let scopes = if !pkg.is_monorepo() {
                None
            } else if scope.is_none() {
                let options = pkg.packages()?;
                let indexes = MultiSelect::new()
                    .with_prompt("Select the packages to release:")
                    .items(&options)
                    .clear(true)
                    .interact()?;

                Some(
                    indexes
                        .into_iter()
                        .map(|idx| options[idx].clone())
                        .collect(),
                )
            } else {
                Some(vec![scope.as_ref().unwrap().clone()])
            };

            if let Some(scopes) = scopes {
                for scope in scopes {
                    changelog.set_scope(Some(scope.clone()));

                    // TODO: properly handle the version in a monorepo per package. This means if package A
                    // is on 0.1.2 and package B is on 0.2.1 and we create a "minor" release than this
                    // should happen:
                    // - package A 0.1.2 -> 0.2.0
                    // - package B 0.2.1 -> 0.3.0
                    output(format!("Releasing {}", version.to_string().green().bold()));
                    changelog.parse_contents()?.release(version)?;

                    if *with_npm {
                        // Commit the CHANGELOG.md file
                        Git::new(Some(&args.pwd))?
                            .add(changelog.file_path_str())?
                            .commit(&format!("update changelog ({})", scope))?;

                        // Execute npm version <version>
                        Npm::new(Some(&args.pwd))?.version(version)?;
                    }
                }
            } else {
                // TODO: properly handle the version in a monorepo per package. This means if package A
                // is on 0.1.2 and package B is on 0.2.1 and we create a "minor" release than this
                // should happen:
                // - package A 0.1.2 -> 0.2.0
                // - package B 0.2.1 -> 0.3.0
                output(format!("Releasing {}", version.to_string().green().bold()));
                changelog.parse_contents()?.release(version)?;

                if *with_npm {
                    // Commit the CHANGELOG.md file
                    Git::new(Some(&args.pwd))?
                        .add(changelog.file_path_str())?
                        .commit("update changelog")?;

                    // Execute npm version <version>
                    Npm::new(Some(&args.pwd))?.version(version)?;
                }
            }

            Ok(())
        }
        Commands::List { amount, all } => changelog.parse_contents()?.list(amount, all),
    }
}
