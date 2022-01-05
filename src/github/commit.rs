use crate::github::github_url::GitHubURL;
use crate::github::repo::Repo;
use crate::graphql::graphql;
use serde_json::json;
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(Debug)]
pub struct Commit {
    hash: String,
    short_hash: String,
    title: String,
    repo: Repo,
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ([{}](https://github.com/{}/{}/commit/{}))",
            self.title, self.short_hash, self.repo.org, self.repo.repo, self.hash
        )
    }
}

impl FromStr for Commit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: GitHubURL = s.parse()?;
        let commit = url.parts.get("commit").expect("Missing commit hash in URL");

        let data = json!({
            "query": include_str!("./graphql/commit-info/query.graphql"),
            "variables": {
                "org": url.repo.org,
                "repo": url.repo.repo,
                "hash": commit
            }
        });

        let json = graphql(data)?;

        let title = json["data"]["repository"]["object"]["title"]
            .as_str()
            .unwrap();
        let short_hash = json["data"]["repository"]["object"]["short_hash"]
            .as_str()
            .unwrap();

        Ok(Self {
            hash: commit.to_string(),
            short_hash: short_hash.to_string(),
            title: title.to_string(),
            repo: url.repo,
        })
    }
}
