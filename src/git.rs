use std::process::Command;

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn long_hash(pwd: &str, hash: &str) -> Result<String, std::io::Error> {
        Self::exec(pwd, vec!["log", "-1", "--format=%H", hash])
    }

    pub fn short_hash(pwd: &str, hash: &str) -> Result<String, std::io::Error> {
        Self::exec(pwd, vec!["log", "-1", "--format=%S", hash])
    }

    pub fn commit_message(pwd: &str, hash: &str) -> Result<String, std::io::Error> {
        match Self::exec(pwd, vec!["log", "-1", "--format=%B", hash]) {
            Ok(msg) => {
                if msg.is_empty() {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No commit message found",
                    ))
                } else {
                    let msg = msg.trim().split_once("\n").unwrap().0;

                    Ok(msg.to_string())
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn exec(pwd: &str, args: Vec<&str>) -> Result<String, std::io::Error> {
        let mut cmd = Command::new("git");

        cmd.current_dir(pwd);

        for arg in args {
            cmd.arg(arg);
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stdout = stdout.trim();
                let stdout = stdout.to_string();

                Ok(stdout)
            }
            Err(e) => Err(e),
        }
    }
}
