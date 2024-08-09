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
use crate::markdown::{ast::Node, tokens::MarkdownToken};
use crate::npm::{Npm, Options};
use crate::output::{output, output_indented, output_title};
use crate::package::{PackageJSON, SemVer};
use crate::rich_edit::rich_edit;
use clap::{Parser, Subcommand};
use color_eyre::eyre::{eyre, Result};
use colored::*;
use dialoguer::MultiSelect;
use std::{collections::HashMap, fmt::Debug, fs, path::PathBuf};

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
                        .map(|package| package.display_name())
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

    match &args.command {
        Commands::Init => {
            match scopes {
                Some(scopes) => {
                    let mut messages: Vec<_> = vec![];
                    for scope in scopes {
                        let mut changelog = Changelog::new(scope.pwd(), &args.filename)?;
                        messages.push(changelog.init()?);
                    }

                    output(
                        messages
                            .iter()
                            .map(|msg| format!("- {}", msg))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                }
                None => {
                    let mut changelog = Changelog::new(&pwd, &args.filename)?;
                    output(changelog.init()?);
                }
            }

            Ok(())
        }
        Commands::Add {
            link,
            name,
            message,
            commit,
            edit,
        }
        | Commands::Fix {
            link,
            name,
            message,
            commit,
            edit,
        }
        | Commands::Change {
            link,
            name,
            message,
            commit,
            edit,
        }
        | Commands::Remove {
            link,
            name,
            message,
            commit,
            edit,
        }
        | Commands::Deprecate {
            link,
            name,
            message,
            commit,
            edit,
        } => {
            match &scopes {
                Some(scopes) => {
                    let mut output_messages: HashMap<PathBuf, Vec<String>> = HashMap::default();

                    for package in scopes {
                        let mut changelog = Changelog::new(package.pwd(), &args.filename)?;

                        let messages = if let Some(message) = message {
                            changelog.add_list_item_to_section(
                                name,
                                &message.to_string(),
                                *edit,
                                Some(package),
                            );
                            vec![message.to_string()]
                        } else if let Some(link) = link {
                            let data: GitHubInfo = link.parse().unwrap();
                            changelog.add_list_item_to_section(
                                name,
                                &data.to_string(),
                                *edit,
                                Some(package),
                            );
                            vec![data.to_string()]
                        } else {
                            let preface = &format!(
                                include_str!("./fixtures/add_entry.txt"),
                                name.to_lowercase(),
                            );

                            let data = match rich_edit(Some(preface)) {
                                Some(data) => {
                                    let data = data.trim();
                                    let data: Vec<_> = data
                                        .lines()
                                        .map(|line| line.trim())
                                        .filter(|line| !line.is_empty())
                                        .filter(|line| !line.starts_with('#'))
                                        .map(|line| line.to_string())
                                        .collect();

                                    for line in &data {
                                        changelog.add_list_item_to_section(
                                            name,
                                            line,
                                            *edit,
                                            Some(package),
                                        );
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

                        output_messages.insert(package.pwd().to_path_buf(), messages);

                        changelog.persist()?;
                    }

                    if *commit {
                        // Commit the CHANGELOG.md file
                        let g = Git::new(Some(&pwd))?;

                        for package in scopes {
                            let path = package.pwd().join(&args.filename);
                            if let Some(path) = path.to_str() {
                                g.add(path)?;
                            }
                        }

                        g.commit("update changelog")?;
                    }

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

                    for package in scopes {
                        output_indented(format!("{}", package.name().white().dimmed()));
                        eprintln!();
                        let messages = output_messages.get(&package.pwd().to_path_buf()).unwrap();
                        let changelog = Changelog::new(package.pwd(), &args.filename)?;

                        if let Some(node) =
                            changelog.get_contents_of_section_scope(None, Some(package))
                        {
                            let mut text = node.to_string();

                            for message in messages {
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
                    let mut changelog = Changelog::new(&pwd, &args.filename)?;

                    let messages = if let Some(message) = message {
                        changelog.add_list_item_to_section(name, &message.to_string(), *edit, None);
                        vec![message.to_string()]
                    } else if let Some(link) = link {
                        let data: GitHubInfo = link.parse().unwrap();
                        changelog.add_list_item_to_section(name, &data.to_string(), *edit, None);
                        vec![data.to_string()]
                    } else {
                        let preface = &format!(
                            include_str!("./fixtures/add_entry.txt"),
                            name.to_lowercase()
                        );

                        let data = match rich_edit(Some(preface)) {
                            Some(data) => {
                                let data = data.trim();
                                let data: Vec<_> = data
                                    .lines()
                                    .map(|line| line.trim())
                                    .filter(|line| !line.is_empty())
                                    .filter(|line| !line.starts_with('#'))
                                    .map(|line| line.to_string())
                                    .collect();

                                for line in &data {
                                    changelog.add_list_item_to_section(name, line, *edit, None);
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

                    output(format!(
                        "Added a new entry to the {} section:",
                        name.blue().bold()
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
                        Git::new(Some(&pwd))?
                            .add(changelog.file_path_str())?
                            .commit("update changelog")?;
                    }
                }
            };

            Ok(())
        }
        Commands::Notes { version } => {
            match scopes {
                Some(scopes) => {
                    for package in scopes {
                        let message = Changelog::new(package.pwd(), &args.filename)?
                            .notes(version.as_ref())
                            .unwrap_or_else(|err| err.to_string().red().to_string());

                        output_title(
                            match version {
                                Some(version) => format!(
                                    "Notes for {}, {}",
                                    package.name().white().dimmed(),
                                    version.to_lowercase().blue()
                                ),
                                None => format!(
                                    "Notes for {}, {}",
                                    package.name().white().dimmed(),
                                    "latest".blue()
                                ),
                            },
                            message,
                        )
                    }
                }
                None => {
                    let message = Changelog::new(&pwd, &args.filename)?
                        .notes(version.as_ref())
                        .unwrap_or_else(|err| err.to_string().red().to_string());

                    output_title(
                        match version {
                            Some(version) => format!("Notes for {}", version.to_lowercase().blue()),
                            None => format!("Notes for {}", "latest".blue()),
                        },
                        message,
                    )
                }
            }

            Ok(())
        }
        Commands::Release { version, with_npm } => {
            match &scopes {
                Some(scopes) => {
                    let repo = Git::new(Some(&pwd))?;
                    let mut changelog_commit_messages: Vec<String> = vec![];
                    let mut output_messages: Vec<String> = vec![];

                    for package in scopes {
                        let mut changelog = Changelog::new(package.pwd(), &args.filename)?;

                        let pwd_str = package.pwd().to_str().unwrap();
                        let mut package = package.clone();
                        let package_version = package.version_mut();
                        let version = package_version.change_to(version)?;

                        // TODO: Only release when things changed?
                        // if !changelog.has_changes(&scope) {
                        //     continue;
                        // }

                        output_messages.push(format!(
                            "- Releasing {} for {}",
                            version.to_string().green().bold(),
                            package.name().white().dimmed()
                        ));
                        changelog.release(&version, Some(&package))?;

                        // Add the CHANGELOG.md file, so that we can commit it later.
                        repo.add(changelog.file_path_str())?;

                        if *with_npm {
                            Npm::new(Some(pwd_str))?.version_options(
                                &version,
                                Options {
                                    no_git_tag_version: true,
                                },
                            )?;

                            // Add the `package-lock.json` file
                            let pkg_lock = pwd.join("package-lock.json");
                            if pkg_lock.exists() {
                                repo.add(pkg_lock.to_str().unwrap())?;
                            }

                            // Add the `package.json` file
                            repo.add(pwd.join("package.json").to_str().unwrap())?;

                            // Commit
                            repo.commit(&format!("{} - {}", &version, &package.name()))?;

                            // Generate a tag
                            repo.tag(&format!("{}@v{}", &package.name(), &version))?;
                        } else {
                            changelog_commit_messages.push(format!(
                                "- Released `{}` for `{}`",
                                version,
                                package.name(),
                            ));
                        }
                    }

                    // Commit the CHANGELOG.md file
                    if !changelog_commit_messages.is_empty() {
                        let _ = &repo.commit(&format!(
                            "update changelog\n\n{}",
                            changelog_commit_messages.join("\n")
                        ))?;
                    }

                    output(output_messages.join("\n"));
                }
                None => {
                    let mut changelog = Changelog::new(&pwd, &args.filename)?;

                    let version: SemVer = version.parse()?;
                    output(format!("Releasing {}", &version.to_string().green().bold()));
                    changelog.release(&version, None)?;

                    if *with_npm {
                        // Commit the CHANGELOG.md file
                        let repo = Git::new(Some(&pwd))?;
                        repo.add(changelog.file_path_str())?;

                        // Execute npm version <version>
                        Npm::new(Some(&args.pwd))?.version_options(
                            &version,
                            Options {
                                no_git_tag_version: true,
                            },
                        )?;

                        // Add the `package-lock.json` file
                        let pkg_lock = pwd.join("package-lock.json");
                        if pkg_lock.exists() {
                            repo.add(pkg_lock.to_str().unwrap())?;
                        }

                        // Add the `package.json` file
                        repo.add(pwd.join("package.json").to_str().unwrap())?;

                        // Commit the version
                        repo.commit(&version.to_string())?;

                        // Let's create a tag!
                        repo.tag(&format!("v{}", &version))?;
                    }
                }
            }

            Ok(())
        }
        Commands::List { amount, all } => {
            let amount = match &all {
                true => Amount::All,
                false => *amount,
            };

            match scopes {
                Some(scopes) => {
                    for package in scopes {
                        let message = Changelog::new(package.pwd(), &args.filename)?
                            .list(amount)
                            .unwrap_or_else(|err| err.to_string().red().to_string());

                        output_title(
                            format!("Releases for {}", package.name().white().dimmed()),
                            message,
                        )
                    }
                }
                None => {
                    output(Changelog::new(&pwd, &args.filename)?.list(amount)?);
                }
            }

            Ok(())
        }
    }
}
