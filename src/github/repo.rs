use crate::git::Git;
use crate::output::output;

#[derive(Debug)]
pub struct Repo {
    pub org: String,
    pub repo: String,
}

impl Repo {
    pub fn new(org: String, repo: String) -> Self {
        Self { org, repo }
    }

    pub fn from_git_repo(pwd: &str) -> Self {
        match Git::exec(pwd, vec!["config", "--get", "remote.origin.url"]) {
            Ok(output) => {
                let output = output.replace(".git", "");

                let parts = output.split(':').collect::<Vec<&str>>()[1]
                    .split('/')
                    .collect::<Vec<&str>>();

                let owner = parts.get(0).unwrap().to_string();
                let repo = parts.get(1).unwrap().to_string();

                Self::new(owner, repo)
            }
            Err(e) => {
                output(format!("Failed running git: {}", e.to_string()));
                std::process::exit(1);
            }
        }
    }
}
