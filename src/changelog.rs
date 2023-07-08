use crate::{git::Git, github::repo::Repo, rich_edit, MarkdownToken, Node, PackageJSON, SemVer};
use chrono::prelude::*;
use color_eyre::eyre::{eyre, Result};
use colored::*;
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

const UNRELEASED_HEADING: &str = "Unreleased";

#[derive(Debug, Clone)]
pub struct Changelog {
    pwd: PathBuf,
    file_path: PathBuf,
    root: Node,
}

impl Changelog {
    pub fn new(pwd: &Path, filename: &str) -> Result<Self> {
        let pwd = fs::canonicalize(pwd)?;
        let file_path = pwd.join(filename);
        let root = match std::fs::metadata(&file_path).is_ok() {
            true => fs::read_to_string(&file_path)?.parse()?,
            false => Node::empty(),
        };

        Ok(Changelog {
            pwd,
            file_path,
            root,
        })
    }

    pub fn file_path_str(&self) -> &str {
        self.file_path.to_str().unwrap()
    }

    pub fn relative_path(&self) -> Result<String> {
        Ok(self
            .file_path_str()
            .replace(std::env::current_dir()?.to_str().unwrap(), "."))
    }

    pub fn unreleased_heading(&self, scope: Option<&PackageJSON>) -> String {
        match scope {
            Some(scope) => format!("[{} - {}]", UNRELEASED_HEADING, scope.name()),
            None => format!("[{}]", UNRELEASED_HEADING),
        }
    }

    pub fn init(&mut self) -> Result<String> {
        let meta = fs::metadata(&self.file_path);

        if meta.is_ok() {
            return Ok(format!(
                "Changelog already exists at: {}",
                &self.relative_path()?.white().dimmed()
            ));
        }

        if !Git::new(Some(&self.pwd))?.is_git_repo() {
            return Ok(format!(
                "Not a git repository: {}",
                self.pwd.to_str().unwrap().white().dimmed()
            ));
        }

        let date = Local::now().format("%Y-%m-%d");
        let repo = Repo::from_git_repo(&self.pwd)?;

        self.root = include_str!("./fixtures/changelog.md")
            .to_string()
            .replace("<date>", &date.to_string())
            .replace("<owner>", &repo.org)
            .replace("<repo>", &repo.repo)
            .parse()?;

        self.persist().map(|_| {
            format!(
                "Created new changelog file at: {}",
                &self.relative_path().unwrap().white().dimmed()
            )
        })
    }

    pub fn persist(&self) -> Result<()> {
        fs::write(&self.file_path, self.root.to_string() + "\n").map_err(|e| eyre!(e))
    }

    fn find_latest_version(&self) -> Option<&str> {
        if let Some(node) = self.root.find_node(|node| {
            if let Some(MarkdownToken::Reference(name, _)) = &node.data {
                !name.to_lowercase().starts_with("unreleased")
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
    fn add_list_item_to_section_scope(
        &mut self,
        section_name: &str,
        item: String,
        scope: Option<&PackageJSON>,
    ) {
        let unreleased_heading = self.unreleased_heading(scope);
        let unreleased = self.root.find_node_mut(|node| match &node.data {
            Some(MarkdownToken::H2(name)) => name.eq_ignore_ascii_case(&unreleased_heading),
            _ => false,
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

            let section = unreleased.find_node_mut(|node| match &node.data {
                Some(MarkdownToken::H3(name)) => name.eq_ignore_ascii_case(section_name),
                _ => false,
            });

            if let Some(section) = section {
                let ul = section
                    .find_node_mut(|node| matches!(&node.data, Some(MarkdownToken::UnorderedList)));

                if let Some(ul) = ul {
                    let li = Node::from_token(MarkdownToken::ListItem(item, 0));

                    ul.add_child(li);
                } else {
                    let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                    let li = Node::from_token(MarkdownToken::ListItem(item, 0));

                    ul.add_child(li);

                    section.add_child(ul);
                }
            } else {
                let mut h3 = Node::from_token(MarkdownToken::H3(section_name.to_string()));
                let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                let li = Node::from_token(MarkdownToken::ListItem(item, 0));

                ul.add_child(li);
                h3.add_child(ul);

                unreleased.add_child(h3);
            }
        } else {
            let unreleased_heading = self.unreleased_heading(scope);
            let mut section = Node::from_token(MarkdownToken::H2(unreleased_heading));
            let mut h3 = Node::from_token(MarkdownToken::H3(section_name.to_string()));
            let mut ul = Node::from_token(MarkdownToken::UnorderedList);
            let li = Node::from_token(MarkdownToken::ListItem(item, 0));

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

    pub fn edit(&self, name: &str, message: &str, scope: Option<&PackageJSON>) -> Option<String> {
        let contents = &format!(
            include_str!("./fixtures/edit_entry.txt"),
            match scope {
                Some(scope) => format!("# Current scope: '{}'\n\n", scope.name()),
                None => "".to_string(),
            },
            message,
            name.to_lowercase(),
        );

        match rich_edit(Some(contents)) {
            Some(data) => {
                let data = data.trim();
                let data = data
                    .lines()
                    .into_iter()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .filter(|line| !line.starts_with('#'))
                    .map(|line| line.to_string())
                    .collect::<Vec<String>>()
                    .join("\n");

                if data.is_empty() {
                    None
                } else {
                    Some(data)
                }
            }
            None => None,
        }
    }

    pub fn add_list_item_to_section(
        &mut self,
        section_name: &str,
        item: &str,
        edit: bool,
        scope: Option<&PackageJSON>,
    ) {
        self.add_list_item_to_section_scope(
            section_name,
            match edit {
                true => match self.edit(section_name, item, scope) {
                    Some(data) => data,
                    None => item.to_string(),
                },
                false => item.to_string(),
            },
            None,
        );
    }

    pub fn get_contents_of_section_scope(
        &self,
        name: Option<&String>,
        scope: Option<&PackageJSON>,
    ) -> Option<Node> {
        let node = self.root.find_node(|node| {
            if let Some(MarkdownToken::H2(section_name)) = &node.data {
                match name {
                    Some(name) => {
                        if name.eq_ignore_ascii_case("latest") {
                            !section_name.eq_ignore_ascii_case(&self.unreleased_heading(scope))
                        } else {
                            match scope {
                                Some(scope) if !scope.is_root() => {
                                    section_name.to_lowercase().starts_with(&format!(
                                        "[{}@v{}]",
                                        scope.name(),
                                        name.to_lowercase()
                                    ))
                                }
                                _ => section_name
                                    .to_lowercase()
                                    .starts_with(&format!("[{}]", name.to_lowercase())),
                            }
                        }
                    }
                    None => {
                        if section_name.eq_ignore_ascii_case(&self.unreleased_heading(scope)) {
                            node.find_node(|node| matches!(&node.data, Some(MarkdownToken::H3(_))))
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

    pub fn get_contents_of_section(&self, name: &Option<String>) -> Option<Node> {
        self.get_contents_of_section_scope(name.as_ref(), None)
    }

    fn notes_scope(&self, version: Option<&String>, scope: Option<&PackageJSON>) -> Result<String> {
        Ok(
            if let Some(node) = self.get_contents_of_section_scope(version, scope) {
                node.to_string()
            } else {
                match version {
                    Some(version) => format!(
                        "Couldn't find notes for version: {} {}",
                        version.blue().bold(),
                        scope
                            .map(|scope| format!("({})", scope.name().white().dimmed()))
                            .unwrap_or_else(|| "".to_string())
                    ),
                    None => format!(
                        "Couldn't find notes for version: {} {}",
                        "<unknown>".blue().bold(),
                        scope
                            .map(|scope| format!("({})", scope.name().white().dimmed()))
                            .unwrap_or_else(|| "".to_string())
                    ),
                }
            },
        )
    }

    pub fn notes(&self, version: Option<&String>) -> Result<String> {
        self.notes_scope(version, None)
    }

    pub fn list(&self, amount: Amount) -> Result<String> {
        let releases = self
            .root
            .filter_nodes(|node| matches!(&node.data, Some(MarkdownToken::Reference(_, _))))
            .iter()
            .filter_map(|node| node.data.as_ref())
            .take(match amount {
                Amount::All => std::usize::MAX,
                Amount::Value(x) => x,
            })
            .map(|token| match token {
                MarkdownToken::Reference(name, link) => format!("- {:15} {}", name, link),
                _ => panic!("Expected a reference"),
            })
            .collect::<Vec<_>>()
            .join("\n");

        if releases.is_empty() {
            Ok("There are no releases yet.".to_string())
        } else {
            Ok(releases)
        }
    }

    pub fn release(&mut self, version: &SemVer, scope: Option<&PackageJSON>) -> Result<()> {
        let date = Local::now().format("%Y-%m-%d");

        let unreleased_heading = self.unreleased_heading(None);

        if let Some(unreleased) = self.root.find_node_mut(|node| {
            if let Some(MarkdownToken::H2(name)) = &node.data {
                name.eq_ignore_ascii_case(&unreleased_heading)
            } else {
                false
            }
        }) {
            // Convert to the new version
            unreleased.rename_heading(&format!("[{}] - {}", version, date));

            // Insert new [Unreleased] section at the top
            let mut new_unreleased =
                Node::from_token(MarkdownToken::H2(unreleased_heading.clone()));
            let mut ul = Node::from_token(MarkdownToken::UnorderedList);
            let li = Node::from_token(MarkdownToken::ListItem("Nothing yet!".to_string(), 0));

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
                    if let Some(unreleased_reference) =
                        self.root.find_node_mut(|node| match &node.data {
                            Some(MarkdownToken::Reference(name, _)) => name.eq_ignore_ascii_case(
                                &unreleased_heading[1..unreleased_heading.len() - 1],
                            ),
                            _ => false,
                        })
                    {
                        if let Some(MarkdownToken::Reference(name, link)) =
                            &unreleased_reference.data
                        {
                            let (updated_link, new_link) = (
                                link.clone().replace(old_version, &version.to_string()),
                                link.clone().replace(
                                    "HEAD",
                                    &match scope {
                                        Some(scope) if !scope.is_root() => {
                                            format!("{}@v{}", scope.name(), version)
                                        }
                                        _ => format!("v{}", version),
                                    },
                                ),
                            );

                            // Update unreleased_reference
                            unreleased_reference.data =
                                Some(MarkdownToken::Reference(name.to_string(), updated_link));

                            // Insert new version reference
                            let new_version_reference = Node::from_token(MarkdownToken::Reference(
                                version.to_string(),
                                new_link,
                            ));

                            match self.root.children.iter().position(|node| match &node.data {
                                Some(MarkdownToken::Reference(name, _)) => {
                                    !name.to_lowercase().starts_with("unreleased")
                                }
                                _ => false,
                            }) {
                                Some(idx) => self.root.add_child_at(idx, new_version_reference),
                                None => self.root.add_child(new_version_reference),
                            }
                        }
                    }
                }
                None => {
                    return Err(eyre!(
                        "Couldn't find latest version, is your CHANGELOG.md formatted correctly?"
                    ));
                }
            }
        }

        self.persist()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Amount {
    All,
    Value(usize),
}

impl FromStr for Amount {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "all" => Amount::All,
            _ => Amount::Value(s.parse::<usize>().map_err(|_| "Invalid amount")?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_find_the_latest_version() {
        let c = Changelog {
            root: Node::from_str(include_str!("../CHANGELOG.md")).unwrap(),
            pwd: PathBuf::default(),
            file_path: PathBuf::default(),
        };

        let latest_version = c.find_latest_version();
        assert_eq!(latest_version, Some("0.1.0"));
    }

    #[test]
    fn it_should_get_the_contents_of_a_section() {
        let c = Changelog {
            root: Node::from_str(include_str!("../CHANGELOG.md")).unwrap(),
            pwd: PathBuf::default(),
            file_path: PathBuf::default(),
        };

        let unreleased_section = c.get_contents_of_section(&Some("unreleased".to_string()));
        assert!(unreleased_section.is_some());

        let unreleased_section = unreleased_section.unwrap();
        assert_eq!(
            unreleased_section,
            Node::from_str("- Nothing yet!").unwrap()
        );

        let first_release = c.get_contents_of_section(&Some("0.1.0".to_string()));
        assert!(first_release.is_some());

        let first_release = first_release.unwrap();
        assert_eq!(
            first_release,
            Node::from_str("### Added\n- Everything!").unwrap()
        );
    }

    #[test]
    fn it_should_generate_a_list_of_releases() {
        let c = Changelog {
            root: Node::from_str(include_str!("../CHANGELOG.md")).unwrap(),
            pwd: PathBuf::default(),
            file_path: PathBuf::default(),
        };

        assert_eq!(
            c.list(Amount::All).unwrap(),
            vec![
                "- unreleased      https://github.com/RobinMalfait/changelog/compare/v0.1.0...HEAD",
                "- 0.1.0           https://github.com/RobinMalfait/changelog/releases/tag/v0.1.0",
            ]
            .join("\n")
        );
    }

    #[test]
    fn it_should_be_possible_to_add_something_to_a_section() {
        let mut c = Changelog {
            root: Node::from_str(include_str!("../CHANGELOG.md")).unwrap(),
            pwd: PathBuf::default(),
            file_path: PathBuf::default(),
        };

        let unreleased_section = c.get_contents_of_section(&Some("unreleased".to_string()));
        assert!(unreleased_section.is_some());
        let unreleased_section = unreleased_section.unwrap();

        assert_eq!(
            unreleased_section,
            Node {
                data: None,
                children: vec![Node {
                    data: Some(MarkdownToken::UnorderedList,),
                    children: vec![Node {
                        data: Some(MarkdownToken::ListItem("Nothing yet!".to_string(), 0)),
                        children: vec![],
                    }],
                }],
            }
        );

        c.add_list_item_to_section("Added", "Something new", false, None);

        let unreleased_section = c.get_contents_of_section(&Some("unreleased".to_string()));
        assert!(unreleased_section.is_some());
        let unreleased_section = unreleased_section.unwrap();

        assert_eq!(
            unreleased_section,
            Node {
                data: None,
                children: vec![Node {
                    data: Some(MarkdownToken::H3("Added".to_string())),
                    children: vec![Node {
                        data: Some(MarkdownToken::UnorderedList),
                        children: vec![Node {
                            data: Some(MarkdownToken::ListItem("Something new".to_string(), 0)),
                            children: vec![],
                        }],
                    }],
                }],
            }
        );

        c.add_list_item_to_section("Added", "Something newer", false, None);

        let unreleased_section = c.get_contents_of_section(&Some("unreleased".to_string()));
        assert!(unreleased_section.is_some());
        let unreleased_section = unreleased_section.unwrap();

        assert_eq!(
            unreleased_section,
            Node {
                data: None,
                children: vec![Node {
                    data: Some(MarkdownToken::H3("Added".to_string())),
                    children: vec![Node {
                        data: Some(MarkdownToken::UnorderedList),
                        children: vec![
                            Node {
                                data: Some(MarkdownToken::ListItem("Something new".to_string(), 0)),
                                children: vec![],
                            },
                            Node {
                                data: Some(MarkdownToken::ListItem(
                                    "Something newer".to_string(),
                                    0
                                )),
                                children: vec![],
                            }
                        ],
                    }],
                }],
            }
        );
    }
}
