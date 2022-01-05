use crate::github::github_url::GitHubURL;
use crate::github::repo::Repo;
use crate::graphql::graphql;
use serde_json::json;
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(Debug)]
pub struct Discussion {
    number: usize,
    title: String,
    repo: Repo,
}

impl Display for Discussion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ([#{}](https://github.com/{}/{}/discussions/{}))",
            self.title, self.number, self.repo.org, self.repo.repo, self.number
        )
    }
}

impl FromStr for Discussion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: GitHubURL = s.parse()?;

        let discussion: usize = url
            .parts
            .get("discussion")
            .expect("Missing discussion in URL")
            .parse()
            .map_err(|_| "Invalid discussion number")?;

        let data = json!({
            "query": include_str!("./graphql/discussion-info/query.graphql"),
            "variables": {
                "org": url.repo.org,
                "repo": url.repo.repo,
                "discussion": discussion
            }
        });

        let json = graphql(data)?;

        let title = json["data"]["repository"]["discussion"]["title"]
            .as_str()
            .unwrap();

        Ok(Self {
            number: discussion,
            title: title.to_string(),
            repo: url.repo,
        })
    }
}
