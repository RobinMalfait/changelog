use color_eyre::eyre::{eyre, Error, Result};
use colored::*;
use glob::glob;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Semantic Versioning 2.0.0: https://semver.org
#[derive(Serialize, Debug, Clone)]
pub struct SemVer {
    /// Version when you make incompatible API changes
    major: u64,

    /// Version when you add functionality in a backwards compatible manner
    minor: u64,

    /// Version when you make backwards compatible bug fixes
    patch: u64,

    /// A pre-release version MAY be denoted by appending a hyphen and a series of dot separated
    /// identifiers immediately following the patch version.
    pre_release: Option<String>,
}

impl SemVer {
    pub fn new(major: u64, minor: u64, patch: u64, pre_release: Option<String>) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release,
        }
    }

    pub fn change_to(&mut self, version: &str) -> Result<Self, Error> {
        let version = match version {
            "major" => self.new_major(),
            "minor" => self.new_minor(),
            "patch" => self.new_patch(),
            "infer" => self.clone(),
            _ => version.parse::<Self>()?,
        };

        *self = version;

        Ok(self.clone())
    }
}

impl SemVer {
    fn new_major(&self) -> Self {
        Self::new(self.major + 1, 0, 0, None)
    }

    fn new_minor(&self) -> Self {
        Self::new(self.major, self.minor + 1, 0, None)
    }

    fn new_patch(&self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1, None)
    }
}

impl Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pre_release) = &self.pre_release {
            write!(
                f,
                "{}.{}.{}-{}",
                self.major, self.minor, self.patch, pre_release
            )
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

impl FromStr for SemVer {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "major" => Ok(PackageJSON::from_current_directory()?.version.new_major()),
            "minor" => Ok(PackageJSON::from_current_directory()?.version.new_minor()),
            "patch" => Ok(PackageJSON::from_current_directory()?.version.new_patch()),
            "infer" => Ok(PackageJSON::from_current_directory()?.version),
            _ => {
                let (s, pre_release) = match s.split_once('-') {
                    Some((s, pre_release)) => (s, Some(pre_release.to_owned())),
                    None => (s, None),
                };
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

                Ok(Self::new(major, minor, patch, pre_release))
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageJSON {
    // Meta data
    #[serde(skip)]
    pwd: PathBuf,
    #[serde(skip)]
    is_root: bool,

    // Actual PackageJSON data
    name: String,
    version: SemVer,
    workspaces: Option<Vec<String>>,
}

impl PackageJSON {
    pub fn from_directory(dir: &Path) -> Result<Self> {
        let package_json_path = dir.join("package.json");
        let contents = std::fs::read_to_string(package_json_path)?;
        serde_json::from_str::<Self>(&contents)
            .map(|mut pkg| {
                pkg.pwd = dir.to_path_buf();
                pkg
            })
            .map_err(|e| eyre!(e))
    }

    pub fn from_root(dir: &Path) -> Result<Self> {
        let package_json_path = dir.join("package.json");
        let contents = std::fs::read_to_string(package_json_path)?;
        serde_json::from_str::<Self>(&contents)
            .map(|mut pkg| {
                pkg.pwd = dir.to_path_buf();
                pkg
            })
            .map_err(|e| eyre!(e))
            .map(|mut root| {
                root.is_root = true;
                root
            })
    }

    pub fn display_name(&self) -> String {
        if self.is_root {
            format!("{} {}", self.name, "(root)".italic().dimmed())
        } else {
            self.name.to_owned()
        }
    }

    pub fn from_current_directory() -> Result<Self> {
        Self::from_directory(&std::env::current_dir()?)
    }

    pub fn pwd(&self) -> &Path {
        &self.pwd
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_root(&self) -> bool {
        self.is_root
    }

    pub fn version_mut(&mut self) -> &mut SemVer {
        &mut self.version
    }

    pub fn is_monorepo(&self) -> bool {
        self.workspaces.is_some()
    }

    pub fn packages(&self) -> Result<Vec<PackageJSON>> {
        let base = &self.pwd;

        let mut packages: Vec<PackageJSON> = vec![PackageJSON::from_root(base)?];

        if let Some(workspaces) = &self.workspaces {
            for workspace_glob in workspaces {
                packages.extend(
                    glob(base.join(workspace_glob).to_str().unwrap())
                        .expect("Failed to read glob pattern")
                        .flatten()
                        .filter(|path| path.is_dir())
                        .filter_map(|path| PackageJSON::from_directory(&path).ok()),
                )
            }
        }

        Ok(packages)
    }
}
