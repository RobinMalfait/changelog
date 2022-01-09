use crate::output::output;
use colored::*;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs::File;
use std::str::FromStr;

/// Semantic Versioning 2.0.0: https://semver.org
#[derive(Serialize, Debug)]
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
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "major" => {
                let pkg = PackageJSON::read("./package.json");

                Ok(Self::new(pkg.version.major + 1, 0, 0))
            }
            "minor" => {
                let pkg = PackageJSON::read("./package.json");

                Ok(Self::new(pkg.version.major, pkg.version.minor + 1, 0))
            }
            "patch" => {
                let pkg = PackageJSON::read("./package.json");

                Ok(Self::new(
                    pkg.version.major,
                    pkg.version.minor,
                    pkg.version.patch + 1,
                ))
            }
            "infer" => {
                let pkg = PackageJSON::read("./package.json");
                Ok(pkg.version)
            }
            _ => {
                let mut parts = s.split('.');
                let major = parts
                    .next()
                    .ok_or_else(|| "Major version is required".to_string())?
                    .parse()
                    .map_err(|_| "Major version must be an integer".to_string())?;

                let minor = parts
                    .next()
                    .ok_or_else(|| "Minor version is required".to_string())?
                    .parse()
                    .map_err(|_| "Minor version must be an integer".to_string())?;

                let patch = parts
                    .next()
                    .ok_or_else(|| "Patch version is required".to_string())?
                    .parse()
                    .map_err(|_| "Patch version must be an integer".to_string())?;

                Ok(Self::new(major, minor, patch))
            }
        }
    }
}

impl<'de> Deserialize<'de> for SemVer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageJSON {
    name: String,
    version: SemVer,
}

impl PackageJSON {
    fn read(path: &str) -> PackageJSON {
        match File::open(path) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(pkg) => pkg,
                Err(e) => {
                    output(format!(
                        "Error while reading {}: {}",
                        "package.json".blue(),
                        e.to_string().red()
                    ));
                    std::process::exit(1);
                }
            },
            Err(e) => {
                output(format!(
                    "Error while reading {}: {}",
                    "package.json".blue(),
                    e.to_string().red()
                ));
                std::process::exit(1);
            }
        }
    }
}
