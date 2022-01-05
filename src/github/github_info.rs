use crate::github::commit::Commit;
use crate::github::discussion::Discussion;
use crate::github::issue::Issue;
use crate::github::pull_request::PullRequest;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug)]
pub enum GitHubInfo {
    PullRequest(PullRequest),
    Commit(Commit),
    Issue(Issue),
    Discussion(Discussion),
}

impl Display for GitHubInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GitHubInfo::PullRequest(pr) => format!("{}", pr),
                GitHubInfo::Commit(commit) => format!("{}", commit),
                GitHubInfo::Issue(issue) => format!("{}", issue),
                GitHubInfo::Discussion(discussion) => format!("{}", discussion),
            }
        )
    }
}

impl FromStr for GitHubInfo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains("/commit/") || s.contains("/commits/") {
            Ok(GitHubInfo::Commit(s.parse()?))
        } else if s.contains("/pull/") || s.contains("/pulls/") {
            Ok(GitHubInfo::PullRequest(s.parse()?))
        } else if s.contains("/issue/") || s.contains("/issues/") {
            Ok(GitHubInfo::Issue(s.parse()?))
        } else if s.contains("/discussion/") || s.contains("/discussions/") {
            Ok(GitHubInfo::Discussion(s.parse()?))
        } else {
            Err(format!("Invalid GitHub URL: {}", s))
        }
    }
}
