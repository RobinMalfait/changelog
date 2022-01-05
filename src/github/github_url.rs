use crate::github::repo::Repo;
use reqwest::Url;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Debug)]
pub struct GitHubURL {
    pub repo: Repo,
    pub parts: HashMap<String, String>,
}

impl FromStr for GitHubURL {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: HashMap<String, String> = HashMap::new();

        let url = Url::parse(s).map_err(|_| "Invalid URL")?;
        let mut segments = url.path()[1..].split('/');

        // Insert known parts
        parts.insert(
            "org".to_string(),
            segments
                .next()
                .expect("URL should contain the organization/owner of the repo")
                .to_string(),
        );
        parts.insert(
            "repo".to_string(),
            segments
                .next()
                .expect("URL should contain the repo")
                .to_string(),
        );

        // Dynamic parts
        while let (Some(key), Some(value)) = (segments.next(), segments.next()) {
            match key {
                "commits" | "commit" => {
                    parts.insert("commit".to_string(), value.to_string());
                }
                "discussions" | "discussion" => {
                    parts.insert("discussion".to_string(), value.to_string());
                }
                "issues" | "issue" => {
                    parts.insert("issue".to_string(), value.to_string());
                }
                _ => {
                    parts.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(Self {
            repo: Repo {
                org: parts.get("org").unwrap().to_string(),
                repo: parts.get("repo").unwrap().to_string(),
            },
            parts,
        })
    }
}
