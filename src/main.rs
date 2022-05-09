mod changelog;
mod git;
mod github;
mod graphql;
mod list_format;
mod markdown;
mod npm;
mod output;
mod package;
mod rich_edit;

use crate::changelog::{Amount, Changelog};
use crate::git::Git;
use crate::github::github_info::GitHubInfo;
use crate::list_format::conjunction;
use crate::markdown::ast::Node;
use crate::markdown::tokens::MarkdownToken;
use crate::npm::Npm;
use crate::output::output;
use crate::output::output_indented;
use crate::package::PackageJSON;
use crate::package::SemVer;
use crate::rich_edit::rich_edit;
use clap::{AppSettings, Parser, Subcommand};
use color_eyre::eyre::{eyre, Result};
use colored::*;
use dialoguer::MultiSelect;
use std::fmt::Debug;
use std::fs;

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

    /// Used in monorepos. Operate on these packages only. You can also pass multiple occurrences.
    /// If none are passed, an interactive prompt will be shown.
    #[clap(
        short,
        long = "scope",
        name = "SCOPE",
        multiple_occurrences = true,
        global = true
    )]
    scopes: Vec<String>,

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

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,

        /// Whether you want to edit the (automated) message after it got fetched from GitHub
        #[clap(short, long)]
        edit: bool,
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

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,

        /// Whether you want to edit the (automated) message after it got fetched from GitHub
        #[clap(short, long)]
        edit: bool,
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

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,

        /// Whether you want to edit the (automated) message after it got fetched from GitHub
        #[clap(short, long)]
        edit: bool,
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

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,

        /// Whether you want to edit the (automated) message after it got fetched from GitHub
        #[clap(short, long)]
        edit: bool,
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

        /// Whether or not to commit the changes
        #[clap(short, long)]
        commit: bool,

        /// Whether you want to edit the (automated) message after it got fetched from GitHub
        #[clap(short, long)]
        edit: bool,
    },

    /// Release a new version
    Release {
        /// The version of the release, which can be one of: "major", "minor", "patch", "infer"
        /// (infer from current package.json version) or an explicit version number like "1.2.3"
        #[clap(default_value = "infer")]
        version: String,

        /// Whether or not to run `npm version <version>` (which in turn updates package.json and
        /// creates a new git tag)
        #[clap(long)]
        with_npm: bool,
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

    // Resolve the current working directory
    let pwd = fs::canonicalize(&args.pwd)?;

    // Resolve the package.json manifest file
    let root_package = PackageJSON::from_directory(&pwd)?;

    // Resolve the current scopes
    let scopes: Option<Vec<PackageJSON>> = if root_package.is_monorepo() {
        let options = root_package.packages()?;

        if args.scopes.is_empty() {
            let resolved_scopes: Vec<PackageJSON> = MultiSelect::new()
                .with_prompt("Select the package(s) to work on")
                .items(
                    &options
                        .iter()
                        .map(|package| package.name())
                        .collect::<Vec<_>>(),
                )
                .clear(true)
                .interact()
                .map(|indexes| {
                    indexes
                        .into_iter()
                        .map(|index| options[index].clone())
                        .collect::<Vec<_>>()
                })?;

            if resolved_scopes.is_empty() {
                return Err(eyre!("No packages selected"));
            }

            Some(resolved_scopes)
        } else {
            let resolved_scopes: Vec<PackageJSON> = options
                .into_iter()
                .filter(|package| args.scopes.iter().any(|scope| package.name().eq(scope)))
                .collect();

            Some(resolved_scopes)
        }
    } else {
        None
    };

    let mut changelog = Changelog::new(&pwd, &args.filename, &scopes)?;

    match &args.command {
        Commands::Init => changelog.init(),
        Commands::Add {
            link,
            message,
            name,
            commit,
            edit,
        }
        | Commands::Fix {
            link,
            message,
            name,
            commit,
            edit,
        }
        | Commands::Change {
            link,
            message,
            name,
            commit,
            edit,
        }
        | Commands::Remove {
            link,
            message,
            name,
            commit,
            edit,
        }
        | Commands::Deprecate {
            link,
            message,
            name,
            commit,
            edit,
        } => {
            changelog.parse_contents()?;

            let messages = if let Some(message) = message {
                changelog.add_list_item_to_section(name, &message.to_string(), edit);
                vec![message.to_string()]
            } else if let Some(link) = link {
                let data: GitHubInfo = link.parse().unwrap();
                changelog.add_list_item_to_section(name, &data.to_string(), edit);
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
                            changelog.add_list_item_to_section(name, line, edit);
                        }

                        if data.is_empty() {
                            None
                        } else {
                            Some(data)
                        }
                    }
                    None => None,
                };

                data.unwrap_or_else(|| {
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
                })
            };

            match &scopes {
                Some(scopes) => {
                    output(format!(
                        "Added a new entry to the {} section {}:",
                        name.blue().bold(),
                        format!(
                            "({})",
                            &conjunction(
                                &scopes.iter().map(|scope| scope.name()).collect::<Vec<_>>()
                            )
                        )
                        .white()
                        .dimmed()
                    ));
                }
                None => {
                    output(format!(
                        "Added a new entry to the {} section:",
                        name.blue().bold()
                    ));
                }
            }

            match &scopes {
                Some(scopes) => {
                    for package in scopes {
                        output_indented(format!("{}", package.name().white().dimmed()));
                        eprintln!();

                        if let Some(node) =
                            changelog.get_contents_of_section_scope(None, Some(package))
                        {
                            let mut text = node.to_string();

                            for message in &messages {
                                text = text.replace(
                                    &format!("- {}", message),
                                    &format!("- {}", message.green().bold()),
                                );
                            }

                            output_indented(text);
                            eprintln!()
                        } else {
                            output_indented("No changes".white().dimmed().italic().to_string());
                            eprintln!()
                        }
                    }
                }
                None => {
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
                }
            }

            changelog.persist()?;

            if *commit {
                // Commit the CHANGELOG.md file
                Git::new(Some(&pwd))?
                    .add(changelog.file_path_str())?
                    .commit("update changelog")?;
            }

            Ok(())
        }
        Commands::Notes { version } => changelog.parse_contents()?.notes(version.as_ref()),
        Commands::Release { version, with_npm } => {
            match &scopes {
                Some(scopes) => {
                    let repo = Git::new(Some(&pwd))?;
                    let changelog = changelog.parse_contents()?;
                    let mut changelog_commit_messages: Vec<String> = vec![];
                    let mut output_messages: Vec<String> = vec![];

                    for package in scopes {
                        let pwd_str = package.pwd().to_str().unwrap();
                        let mut scope = package.clone();
                        let package_version = scope.version_mut();
                        let version = package_version.change_to(version)?;

                        // TODO: Only release when things changed?
                        // if !changelog.has_changes(&scope) {
                        //     continue;
                        // }

                        output_messages.push(format!(
                            "- Releasing {} for {}",
                            version.to_string().green().bold(),
                            scope.name().white().dimmed()
                        ));
                        changelog.release(&version, Some(&scope))?;

                        // Commit the CHANGELOG.md file
                        repo.add(changelog.file_path_str())?;

                        if *with_npm {
                            // TODO: Maybe don't call the npm binary and use `serde` instead?
                            // Execute npm version <version> --no-git-tag-version
                            Npm::new(Some(pwd_str))?.version_options(&version, true)?;

                            // Commit the `package.json` file
                            repo.add(pwd_str)?.commit(&format!(
                                "{} - {}",
                                &version,
                                &scope.name()
                            ))?;

                            // Generate a tag
                            repo.tag(&format!("{}@v{}", &scope.name(), &version))?;
                        } else {
                            changelog_commit_messages.push(format!(
                                "- Released `{}` for `{}`",
                                version,
                                scope.name(),
                            ));
                        }
                    }

                    // Commit the CHANGELOG.md file
                    if !changelog_commit_messages.is_empty() {
                        let _ = &repo.add(changelog.file_path_str())?.commit(&format!(
                            "update changelog\n\n{}",
                            changelog_commit_messages.join("\n")
                        ))?;
                    }

                    output(output_messages.join("\n"));
                }
                None => {
                    let version: SemVer = version.parse()?;
                    output(format!("Releasing {}", &version.to_string().green().bold()));
                    changelog.parse_contents()?.release(&version, None)?;

                    if *with_npm {
                        // Commit the CHANGELOG.md file
                        Git::new(Some(&pwd))?
                            .add(changelog.file_path_str())?
                            .commit("update changelog")?;

                        // Execute npm version <version>
                        Npm::new(Some(&args.pwd))?.version(&version)?;
                    }
                }
            }

            Ok(())
        }
        Commands::List { amount, all } => changelog.parse_contents()?.list(amount, all),
    }
}
