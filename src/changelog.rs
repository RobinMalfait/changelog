use crate::github::repo::Repo;
use crate::output::output;
use crate::MarkdownToken;
use crate::Node;
use crate::SemVer;
use chrono::prelude::*;
use colored::*;
use std::path::PathBuf;
use std::str::FromStr;

const UNRELEASED_HEADING: &str = "Unreleased";

#[derive(Debug, Clone)]
pub struct Changelog {
    file_path: PathBuf,
    root: Node,
}

impl Changelog {
    pub fn new(pwd: &str, filename: &str) -> Self {
        let pwd = std::fs::canonicalize(pwd).expect("File path doesn't seem to exist");
        let file_path = pwd.join(filename);

        let contents = std::fs::read_to_string(&file_path).unwrap_or_else(|_| {
            let date = Local::now().format("%Y-%m-%d");

            let repo = Repo::from_git_repo(pwd.to_str().unwrap());

            include_str!("./fixtures/changelog.md")
                .to_string()
                .replace("<date>", &date.to_string())
                .replace("<owner>", &repo.org)
                .replace("<repo>", &repo.repo)
        });
        let root: Node = contents.parse().expect("Failed to parse changelog file");

        Changelog { file_path, root }
    }

    pub fn init(&self) -> Result<(), std::io::Error> {
        let meta = std::fs::metadata(&self.file_path);

        if meta.is_ok() {
            output(format!(
                "Changelog already exists at: {}",
                self.file_path.to_str().unwrap().white().dimmed()
            ));

            Ok(())
        } else {
            output(format!(
                "Created new changelog file at: {}",
                self.file_path.to_str().unwrap().white().dimmed()
            ));

            self.persist()
        }
    }

    pub fn persist(&self) -> Result<(), std::io::Error> {
        std::fs::write(&self.file_path, self.root.to_string() + "\n")
    }

    pub fn find_latest_version(&self) -> Option<&str> {
        if let Some(node) = self.root.find_node(&|node| {
            if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                !name.eq_ignore_ascii_case(UNRELEASED_HEADING)
            } else {
                false
            }
        }) {
            if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                return Some(name);
            }
        }

        None
    }

    // TODO: This is horrible... refactor this!
    pub fn add_list_item_to_section(&mut self, section_name: &str, item: String) {
        let unreleased = self.root.find_node_mut(&|node| {
            if let Some(MarkdownToken::H2(name)) = &node.data {
                name.eq_ignore_ascii_case(&format!("[{}]", UNRELEASED_HEADING))
            } else {
                false
            }
        });

        if let Some(unreleased) = unreleased {
            // Search for the "Nothing yet!" note, and delete it if it exists.
            let nothing_yet_ul = unreleased
                .children
                .iter_mut()
                .position(|node| matches!(&node.data, Some(MarkdownToken::UnorderedList)));

            if let Some(nothing_yet_ul) = nothing_yet_ul {
                unreleased.children.remove(nothing_yet_ul);
            }

            let section = unreleased.find_node_mut(&|node| {
                if let Some(MarkdownToken::H3(name)) = &node.data {
                    name.eq_ignore_ascii_case(section_name)
                } else {
                    false
                }
            });

            if let Some(section) = section {
                let ul = section.find_node_mut(&|node| {
                    matches!(&node.data, Some(MarkdownToken::UnorderedList))
                });

                if let Some(ul) = ul {
                    let li = Node::from_token(MarkdownToken::ListItem(item));

                    ul.add_child(li);
                } else {
                    let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                    let li = Node::from_token(MarkdownToken::ListItem(item));

                    ul.add_child(li);

                    section.add_child(ul);
                }
            } else {
                let mut h3 = Node::from_token(MarkdownToken::H3(section_name.to_string()));
                let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                let li = Node::from_token(MarkdownToken::ListItem(item));

                ul.add_child(li);
                h3.add_child(ul);

                unreleased.add_child(h3);
            }
        } else {
            let mut section =
                Node::from_token(MarkdownToken::H2(format!("[{}]", UNRELEASED_HEADING)));
            let mut h3 = Node::from_token(MarkdownToken::H3(section_name.to_string()));
            let mut ul = Node::from_token(MarkdownToken::UnorderedList);
            let li = Node::from_token(MarkdownToken::ListItem(item));

            ul.add_child(li);
            h3.add_child(ul);
            section.add_child(h3);

            // Insert "Unreleased" section
            self.root
                .children
                .get_mut(0)
                .expect("Couldn't find main heading, is your CHANGELOG.md formatted correctly?")
                .add_child_at(2, section);
        }
    }

    pub fn get_contents_of_section(&self, name: &Option<String>) -> Option<Node> {
        let node = self.root.find_node(&|node| {
            if let Some(MarkdownToken::H2(section_name)) = &node.data {
                match name {
                    Some(name) => {
                        if name.eq_ignore_ascii_case("latest") {
                            !section_name.eq_ignore_ascii_case(&format!("[{}]", UNRELEASED_HEADING))
                        } else {
                            section_name
                                .to_lowercase()
                                .starts_with(&format!("[{}]", name.to_lowercase()))
                        }
                    }
                    None => {
                        if section_name.eq_ignore_ascii_case(&format!("[{}]", UNRELEASED_HEADING)) {
                            node.find_node(&|node| matches!(&node.data, Some(MarkdownToken::H3(_))))
                                .is_some()
                        } else {
                            true
                        }
                    }
                }
            } else {
                false
            }
        });

        if let Some(node) = node {
            let mut copy = node.clone();
            copy.data = None;

            Some(copy)
        } else {
            None
        }
    }

    pub fn notes(&self, version: &Option<String>) -> Result<(), std::io::Error> {
        if let Some(node) = self.get_contents_of_section(version) {
            output(node.to_string());
        } else {
            match *version {
                Some(ref version) => {
                    output(format!("Couldn't find notes for version: {}", version));
                }
                None => {
                    output("Couldn't find notes for version: <unknown>".to_string());
                }
            }
        }

        Ok(())
    }

    pub fn list(&self, amount: &Amount, all: &bool) -> Result<(), std::io::Error> {
        let releases = self
            .root
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

        if releases.is_empty() {
            output("There are no releases yet.".to_string());
        } else {
            output(releases)
        }

        Ok(())
    }

    pub fn release(&mut self, version: &SemVer) -> Result<(), std::io::Error> {
        let date = Local::now().format("%Y-%m-%d");

        if let Some(unreleased) = self.root.find_node_mut(&|node| {
            if let Some(MarkdownToken::H2(name)) = &node.data {
                name.eq_ignore_ascii_case(&format!("[{}]", UNRELEASED_HEADING))
            } else {
                false
            }
        }) {
            // Convert to the new version
            unreleased.rename_heading(&format!("[{}] - {}", version, date));

            // Insert new [Unreleased] section at the top
            let mut new_unreleased =
                Node::from_token(MarkdownToken::H2(format!("[{}]", UNRELEASED_HEADING)));
            let mut ul = Node::from_token(MarkdownToken::UnorderedList);
            let li = Node::from_token(MarkdownToken::ListItem("Nothing yet!".to_string()));

            ul.add_child(li);
            new_unreleased.add_child(ul);

            self.root
                .children
                .get_mut(0)
                .expect("Couldn't find main heading, is your CHANGELOG.md formatted correctly?")
                .add_child_at(2, new_unreleased);

            // Update references at the bottom
            let c = self.clone();
            match c.find_latest_version() {
                Some(old_version) => {
                    if let Some(unreleased_reference) = self.root.find_node_mut(&|node| {
                        if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                            name.eq_ignore_ascii_case(UNRELEASED_HEADING)
                        } else {
                            false
                        }
                    }) {
                        if let Some(MarkdownToken::Reference(name, link)) =
                            &unreleased_reference.data
                        {
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

                            match self.root.children.iter().position(|node| {
                                if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                                    name.eq_ignore_ascii_case(UNRELEASED_HEADING)
                                } else {
                                    false
                                }
                            }) {
                                Some(idx) => {
                                    self.root.add_child_at(idx + 1, new_version_reference);
                                }
                                None => {
                                    self.root.add_child(new_version_reference);
                                }
                            }
                        }
                    }
                }
                None => {
                    output(
                        "Couldn't find latest version, is your CHANGELOG.md formatted correctly?"
                            .to_string(),
                    );
                    std::process::exit(1);
                }
            }
        }

        self.persist()
    }
}

#[derive(Debug)]
pub enum Amount {
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
