use color_eyre::eyre::{eyre, Error, Result};
use colored::*;
use glob::glob;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::Path;
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "major" => {
                let pkg = PackageJSON::from_current_directory()?;
                Ok(Self::new(pkg.version.major + 1, 0, 0))
            }
            "minor" => {
                let pkg = PackageJSON::from_current_directory()?;
                Ok(Self::new(pkg.version.major, pkg.version.minor + 1, 0))
            }
            "patch" => {
                let pkg = PackageJSON::from_current_directory()?;
                Ok(Self::new(
                    pkg.version.major,
                    pkg.version.minor,
                    pkg.version.patch + 1,
                ))
            }
            "infer" => {
                let pkg = PackageJSON::from_current_directory()?;
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

impl<'de> Deserialize<'de> for SemVer {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
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
    workspaces: Option<Vec<String>>,
}

impl PackageJSON {
    pub fn from_directory(dir: &Path) -> Result<Self> {
        let package_json_path = dir.join("package.json");

        match std::fs::read_to_string(package_json_path) {
            Ok(contents) => match serde_json::from_str::<Self>(&contents) {
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

    pub fn from_current_directory() -> Result<Self> {
        let pwd = std::env::current_dir()?;
        Self::from_directory(&pwd)
    }

    pub fn is_monorepo(&self) -> bool {
        self.workspaces.is_some()
    }

    pub fn packages(&self) -> Result<Vec<String>> {
        // TODO: Get this from `pwd` properly
        let base = std::env::current_dir()?;

        let mut packages = vec![];

        if let Some(workspaces) = &self.workspaces {
            for workspace_glob in workspaces {
                packages.extend(
                    glob(base.join(workspace_glob).to_str().unwrap())
                        .expect("Failed to read glob pattern")
                        .flatten()
                        .filter(|path| path.is_dir())
                        .filter_map(|path| PackageJSON::from_directory(&path).ok())
                        .map(|pkg| pkg.name),
                )
            }
        }

        Ok(packages)
    }
}
