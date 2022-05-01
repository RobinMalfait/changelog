use crate::MarkdownToken;
use color_eyre::eyre::Error;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Node {
    pub data: Option<MarkdownToken>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(data: Option<MarkdownToken>, children: Vec<Node>) -> Self {
        Node { data, children }
    }

    pub fn empty() -> Self {
        Node::new(None, vec![])
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

    pub fn rename_heading(&mut self, name: &str) {
        match self.data {
            Some(MarkdownToken::H1(ref mut heading))
            | Some(MarkdownToken::H2(ref mut heading))
            | Some(MarkdownToken::H3(ref mut heading)) => {
                *heading = name.to_string();
            }
            _ => {}
        }
    }

    pub fn find_node<'a, F>(&'a self, predicate: F) -> Option<&'a Node>
    where
        Self: Sized,
        F: Fn(&'a Node) -> bool + Copy,
    {
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

    pub fn find_node_mut<F>(&mut self, predicate: F) -> Option<&mut Node>
    where
        Self: Sized,
        F: Fn(&Node) -> bool + Copy,
    {
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

    pub fn filter_nodes<'a, F>(&'a self, predicate: F) -> Vec<&'a Node>
    where
        Self: Sized,
        F: Fn(&'a Node) -> bool + Copy,
    {
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
    type Err = Error;

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
    // TODO: Improve converting our tokens to an AST
    let mut root: Vec<Node> = vec![];

    while let Some(token) = tokens.next() {
        root.push(match token {
            MarkdownToken::H1(_) | MarkdownToken::H2(_) | MarkdownToken::H3(_) => {
                Node::new(Some(token.clone()), parse(tokens, Some(token)))
            }
            MarkdownToken::ListItem(_, _) => {
                let mut ul = Node::from_token(MarkdownToken::UnorderedList);
                ul.add_child(Node::from_token(token.clone()));

                while let Some(MarkdownToken::ListItem(_, _)) = &tokens.peek() {
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
