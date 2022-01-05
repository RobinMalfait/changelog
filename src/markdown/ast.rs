use crate::MarkdownToken;
use std::fmt::Display;
use std::str::FromStr;

const UNRELEASED_HEADING: &str = "[Unreleased]";
pub const NEXT_OR_LATEST: &str = "next_or_latest";

#[derive(Debug, Clone)]
pub struct Node {
    pub data: Option<MarkdownToken>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(data: Option<MarkdownToken>, children: Vec<Node>) -> Self {
        Node { data, children }
    }

    pub fn from_token(token: MarkdownToken) -> Self {
        Node::new(Some(token), vec![])
    }

    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn add_child_at(&mut self, index: usize, child: Node) {
        self.children.insert(index, child);
    }

    // TODO: This is horrible... refactor this!
    pub fn add_list_item_to_section(&mut self, section_name: &str, item: String) {
        let unreleased = self.find_node_mut(&|node| {
            if let Some(MarkdownToken::H2(name)) = &node.data {
                name == UNRELEASED_HEADING
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
            let mut section = Node::from_token(MarkdownToken::H2(UNRELEASED_HEADING.to_string()));
            let mut h3 = Node::from_token(MarkdownToken::H3(section_name.to_string()));
            let mut ul = Node::from_token(MarkdownToken::UnorderedList);
            let li = Node::from_token(MarkdownToken::ListItem(item));

            ul.add_child(li);
            h3.add_child(ul);
            section.add_child(h3);

            // Insert "Unreleased" section
            self.children
                .get_mut(0)
                .expect("Couldn't find main heading, is your CHANGELOG.md formatted correctly?")
                .add_child_at(2, section);
        }
    }

    fn find_node<'a>(&'a self, predicate: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
        if predicate(self) {
            return Some(self);
        }

        for child in &self.children {
            if let Some(result) = child.find_node(predicate) {
                return Some(result);
            }
        }

        None
    }

    fn find_node_mut(&mut self, predicate: &dyn Fn(&Node) -> bool) -> Option<&mut Node> {
        if predicate(self) {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(result) = child.find_node_mut(predicate) {
                return Some(result);
            }
        }

        None
    }

    pub fn filter_nodes<'a>(&'a self, predicate: &dyn Fn(&'a Node) -> bool) -> Vec<&'a Node> {
        let mut result: Vec<&'a Node> = vec![];

        if predicate(self) {
            result.push(self);
        }

        for child in &self.children {
            result.extend(child.filter_nodes(predicate));
        }

        result
    }

    fn flatten(&self) -> Vec<&MarkdownToken> {
        let mut result: Vec<&MarkdownToken> = vec![];

        if let Some(MarkdownToken::UnorderedList) = self.data {
            for child in &self.children {
                result.extend(child.flatten());
            }

            result.push(&MarkdownToken::BlankLine);
        } else {
            if let Some(data) = &self.data {
                result.push(data);
            }

            for child in &self.children {
                result.extend(child.flatten())
            }
        }

        result
    }

    pub fn get_contents_of_section(self, name: &str) -> Option<Node> {
        let node = self.find_node(&|node| {
            if let Some(MarkdownToken::H2(section_name)) = &node.data {
                if name.eq_ignore_ascii_case(NEXT_OR_LATEST) {
                    if section_name.eq_ignore_ascii_case(UNRELEASED_HEADING) {
                        node.find_node(&|node| matches!(&node.data, Some(MarkdownToken::H3(_))))
                            .is_some()
                    } else {
                        true
                    }
                } else if name.eq_ignore_ascii_case("latest") {
                    !section_name.eq_ignore_ascii_case(UNRELEASED_HEADING)
                } else {
                    section_name
                        .to_lowercase()
                        .starts_with(&format!("[{}]", name.to_lowercase()))
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
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.flatten()
                .iter()
                .map(|token| token.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

impl FromStr for Node {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = MarkdownToken::lex(s);
        let mut iterator = tokens.iter().peekable();

        Ok(Node::new(None, parse(&mut iterator, None)))
    }
}

/// Parse a list of tokens into a tree of nodes
fn parse(
    tokens: &mut std::iter::Peekable<std::slice::Iter<'_, MarkdownToken>>,
    parent: Option<&MarkdownToken>,
) -> Vec<Node> {
    // TODO: Improve converthing our tokens to an AST
    let mut root: Vec<Node> = vec![];

    while let Some(token) = tokens.next() {
        root.push(match token {
            MarkdownToken::H1(_) | MarkdownToken::H2(_) | MarkdownToken::H3(_) => {
                Node::new(Some(token.clone()), parse(tokens, Some(token)))
            }
            MarkdownToken::ListItem(_) => {
                let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                ul.add_child(Node::from_token(token.clone()));

                while let Some(MarkdownToken::ListItem(_)) = &tokens.peek() {
                    ul.add_child(Node::from_token(tokens.next().unwrap().clone()));
                }

                ul
            }
            _ => Node::from_token(token.clone()),
        });

        if let Some(parent) = parent {
            match (parent, tokens.peek()) {
                (MarkdownToken::H1(_), Some(MarkdownToken::H1(_)))
                | (MarkdownToken::H2(_), Some(MarkdownToken::H2(_)))
                | (MarkdownToken::H2(_), Some(MarkdownToken::H1(_)))
                | (MarkdownToken::H3(_), Some(MarkdownToken::H3(_)))
                | (MarkdownToken::H3(_), Some(MarkdownToken::H2(_)))
                | (MarkdownToken::H3(_), Some(MarkdownToken::H1(_)))
                | (_, Some(MarkdownToken::Reference(_, _))) => {
                    return root;
                }
                _ => {}
            }
        }
    }

    root
}
