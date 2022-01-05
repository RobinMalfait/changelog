use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MarkdownToken {
    H1(String),
    H2(String),
    H3(String),
    Paragraph(String),
    UnorderedList,
    ListItem(String),
    Reference(String, String),
    BlankLine,
}

impl MarkdownToken {
    /// Convert each line to a proper MarkdownToken
    pub fn lex(contents: &str) -> Vec<MarkdownToken> {
        contents
            .split("\n\n")
            .filter(|line| !line.is_empty())
            .flat_map(|group| match &group[..1] {
                "#" | "-" | "[" => group
                    .lines()
                    .map(|line| match line {
                        line if line.starts_with("# ") => MarkdownToken::H1(line[2..].to_string()),
                        line if line.starts_with("## ") => MarkdownToken::H2(line[3..].to_string()),
                        line if line.starts_with("### ") => {
                            MarkdownToken::H3(line[4..].to_string())
                        }
                        line if line.starts_with("- ") => {
                            MarkdownToken::ListItem(line[2..].to_string())
                        }
                        line if line.starts_with('[') => {
                            let mut parts = line.split(": ");
                            let name = parts.next().unwrap();
                            let link = parts.next().unwrap();
                            MarkdownToken::Reference(
                                name[1..(name.len() - 1)].to_string(),
                                link.to_string(),
                            )
                        }
                        _ => MarkdownToken::Paragraph(group.to_string()),
                    })
                    .collect(),
                _ => vec![MarkdownToken::Paragraph(group.to_string())],
            })
            .collect()
    }
}

impl Display for MarkdownToken {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MarkdownToken::H1(line) => writeln!(f, "# {}", line),
            MarkdownToken::H2(line) => writeln!(f, "## {}", line),
            MarkdownToken::H3(line) => writeln!(f, "### {}", line),
            MarkdownToken::Paragraph(line) => writeln!(f, "{}", line),
            MarkdownToken::UnorderedList => Ok(()),
            MarkdownToken::ListItem(line) => write!(f, "- {}", line),
            MarkdownToken::Reference(name, link) => write!(f, "[{}]: {}", name, link),
            MarkdownToken::BlankLine => write!(f, ""),
        }
    }
}
