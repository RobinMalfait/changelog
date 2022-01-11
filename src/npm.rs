use crate::SemVer;
use color_eyre::eyre::{eyre, Result};
use std::process::Command;

#[derive(Debug)]
pub struct NPM {
    pwd: String,
}

impl NPM {
    pub fn new(pwd: Option<&str>) -> Result<Self> {
        match pwd {
            Some(pwd) => Ok(Self {
                pwd: pwd.to_string(),
            }),
            None => Ok(Self {
                pwd: std::env::current_dir()?.display().to_string(),
            }),
        }
    }

    pub fn version(&self, version: &SemVer) -> Result<&Self> {
        self.exec(vec!["version", &version.to_string()])?;
        Ok(&self)
    }

    pub fn exec(&self, args: Vec<&str>) -> Result<String> {
        let mut cmd = Command::new("npm");

        cmd.current_dir(&self.pwd);

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
            Err(e) => Err(eyre!(e)),
        }
    }
}
