use crate::git::Git;
use crate::github::github_url::GitHubURL;
use crate::github::repo::Repo;
use crate::graphql::graphql;
use color_eyre::eyre::Result;
use serde_json::json;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
pub struct Commit {
    hash: String,
    short_hash: String,
    title: String,
    repo: Repo,
}

impl Commit {
    pub fn from_local_commit(pwd: &PathBuf, maybe_hash: &str) -> Result<Self> {
        let repo = Repo::from_git_repo(pwd)?;

        let git = Git::new(Some(pwd))?;

        let long_hash = git.long_hash(maybe_hash)?;
        let short_hash = git.short_hash(maybe_hash)?;
        let title = git.commit_message(maybe_hash)?;

        Ok(Self {
            hash: long_hash,
            short_hash,
            title,
            repo,
        })
    }
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
        match s.parse::<GitHubURL>() {
            Ok(url) => {
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
            Err(_) => {
                // TODO: Get from root
                let pwd = std::fs::canonicalize(".").expect("File path doesn't seem to exist");

                match Commit::from_local_commit(&pwd, s) {
                    Ok(commit) => Ok(commit),
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    }
}
