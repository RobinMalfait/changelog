use color_eyre::eyre::{eyre, Error, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

/// Semantic Versioning 2.0.0: https://semver.org
#[derive(Serialize, Deserialize, Debug)]
pub struct SemVer {
    /// Version when you make incompatible API changes
    major: u64,

    /// Version when you add functionality in a backwards compatible manner
    minor: u64,

    /// Version when you make backwards compatible bug fixes
    patch: u64,
}

impl SemVer {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for SemVer {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "major" => {
                let pkg = PackageJSON::read("./package.json")?;
                Ok(Self::new(pkg.version.major + 1, 0, 0))
            }
            "minor" => {
                let pkg = PackageJSON::read("./package.json")?;
                Ok(Self::new(pkg.version.major, pkg.version.minor + 1, 0))
            }
            "patch" => {
                let pkg = PackageJSON::read("./package.json")?;
                Ok(Self::new(
                    pkg.version.major,
                    pkg.version.minor,
                    pkg.version.patch + 1,
                ))
            }
            "infer" => {
                let pkg = PackageJSON::read("./package.json")?;
                Ok(pkg.version)
            }
            _ => {
                let mut parts = s.split('.');

                let (major, minor, patch) = match (parts.next(), parts.next(), parts.next()) {
                    (Some(major), Some(minor), Some(patch)) => (
                        major.parse::<u64>()?,
                        minor.parse::<u64>()?,
                        patch.parse::<u64>()?,
                    ),
                    (None, _, _) => {
                        return Err(eyre!("{} version is missing", "major".blue().bold()))
                    }
                    (_, None, _) => {
                        return Err(eyre!("{} version is missing", "minor".blue().bold()))
                    }
                    (_, _, None) => {
                        return Err(eyre!("{} version is missing", "patch".blue().bold()))
                    }
                };

                Ok(Self::new(major, minor, patch))
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageJSON {
    version: SemVer,
}

impl PackageJSON {
    fn read(path: &str) -> Result<PackageJSON> {
        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<PackageJSON>(&contents) {
                Ok(pkg) => Ok(pkg),
                Err(e) => Err(eyre!(format!(
                    "Error while reading {}: {}",
                    "package.json".blue(),
                    e.to_string().red()
                ))),
            },
            Err(e) => Err(eyre!(format!(
                "Error while reading {}: {}",
                "package.json".blue(),
                e.to_string().red()
            ))),
        }
    }
}
