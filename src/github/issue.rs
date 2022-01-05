use crate::github::github_url::GitHubURL;
use crate::github::repo::Repo;
use crate::graphql::graphql;
use serde_json::json;
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(Debug)]
pub struct Issue {
    number: usize,
    title: String,
    repo: Repo,
}

impl Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ([#{}](https://github.com/{}/{}/issues/{}))",
            self.title, self.number, self.repo.org, self.repo.repo, self.number
        )
    }
}

impl FromStr for Issue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: GitHubURL = s.parse()?;

        let issue: usize = url
            .parts
            .get("issue")
            .expect("Missing issue in URL")
            .parse()
            .map_err(|_| "Invalid issue number")?;

        let data = json!({
            "query": include_str!("./graphql/issue-info/query.graphql"),
            "variables": {
                "org": url.repo.org,
                "repo": url.repo.repo,
                "issue": issue
            }
        });

        let json = graphql(data)?;

        let title = json["data"]["repository"]["issue"]["title"]
            .as_str()
            .unwrap();

        Ok(Self {
            number: issue,
            title: title.to_string(),
            repo: url.repo,
        })
    }
}
