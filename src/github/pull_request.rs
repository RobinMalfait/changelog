use crate::github::github_url::GitHubURL;
use crate::github::repo::Repo;
use crate::graphql::graphql;
use serde_json::json;
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(Debug)]
pub struct PullRequest {
    number: usize,
    title: String,
    repo: Repo,
}

impl Display for PullRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ([#{}](https://github.com/{}/{}/pull/{}))",
            self.title, self.number, self.repo.org, self.repo.repo, self.number
        )
    }
}

impl FromStr for PullRequest {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: GitHubURL = s.parse()?;

        let pull: usize = url
            .parts
            .get("pull")
            .expect("Missing repo in URL")
            .parse()
            .map_err(|_| "Invalid pull number")?;

        let data = json!({
            "query": include_str!("./graphql/pr-info/query.graphql"),
            "variables": {
                "org": url.repo.org,
                "repo": url.repo.repo,
                "pr": pull
            }
        });

        let json = graphql(data)?;

        let title = json["data"]["repository"]["pullRequest"]["title"]
            .as_str()
            .unwrap();

        Ok(Self {
            number: pull,
            title: title.to_string(),
            repo: url.repo,
        })
    }
}
