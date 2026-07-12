// SPDX-License-Identifier: MIT
// Copyright (c) 2025-2026 Pistonite

use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cu::pre::*;

static CONFIG_NAMES: &[&str] = &["Lisensor.toml", "lisensor.toml"];

/// Try finding the default config files according to the order
/// specified in the documentation (see repo README)
pub fn try_find_default_config_file() -> Option<PathBuf> {
    for x in CONFIG_NAMES {
        if Path::new(x).exists() {
            cu::debug!("found config {x} in current directory");
            return Some(PathBuf::from(x));
        }
    }
    cu::debug!("discovering config in parent directories");
    let mut curr = Path::new(".").normalize().ok()?;
    loop {
        curr = curr.parent_abs().ok()?;
        cu::debug!("looking for config in '{}'", curr.display());
        let mut p = curr.clone();
        for x in CONFIG_NAMES {
            p.push(x);
            if p.exists() {
                cu::debug!("found config '{}'", p.display());
                return Some(p);
            }
            p.pop();
        }
    }
}

/// Config object
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    /// the root directory to run the program
    root: PathBuf,
    /// glob -> (holder, license)
    globs: BTreeMap<String, (Arc<String>, Arc<String>)>,
}

/// Raw config read from a toml config file.
///
/// The format is holder -> glob -> license
#[derive(Deserialize)]
struct TomlConfig(BTreeMap<String, BTreeMap<String, String>>);

impl Config {
    /// Create a config object from a single holder and license,
    /// with multiple glob patterns.
    pub fn new(root: PathBuf, holder: String, license: String, glob_list: Vec<String>) -> Self {
        let holder = Arc::new(holder);
        let license = Arc::new(license);
        let mut globs = BTreeMap::new();
        for glob in glob_list {
            use std::collections::btree_map::Entry;
            match globs.entry(glob) {
                Entry::Vacant(entry) => {
                    entry.insert((Arc::clone(&holder), Arc::clone(&license)));
                }
                Entry::Occupied(entry) => {
                    let glob = entry.key();
                    cu::warn!("glob '{glob}' is specfied multiple times!");
                }
            }
        }
        Self { root, globs }
    }

    /// Build the config by reading the file specified, error if conflicts are detected
    ///
    /// The globs specified in the config file are relative to the parent directory
    /// of `config_path`.
    pub fn build(config_path: &Path) -> cu::Result<Self> {
        let raw = toml::parse::<TomlConfig>(&cu::fs::read_string(config_path)?)?;
        let root = config_path
            .parent_abs()
            .context("failed to get parent path for config")?;
        let mut globs = BTreeMap::new();
        for (holder, table) in raw.0 {
            let holder = Arc::new(holder);
            for (glob, license) in table {
                use std::collections::btree_map::Entry;
                match globs.entry(glob) {
                    Entry::Vacant(entry) => {
                        entry.insert((Arc::clone(&holder), Arc::new(license)));
                    }
                    Entry::Occupied(entry) => {
                        let glob = entry.key();
                        let (curr_holder, curr_license) = entry.get();
                        if *curr_holder == holder && curr_license.deref() == license.as_str() {
                            cu::warn!(
                                "glob '{glob}' specified multiple times in '{}'!",
                                config_path.display()
                            );
                            continue;
                        }
                        cu::error!("conflicting config specified for glob '{glob}':");
                        cu::error!(
                            "- in one config, it has holder '{holder}' and license '{license}'"
                        );
                        cu::error!(
                            "- in another, it has holder '{curr_holder}' and license '{curr_license}'"
                        );
                        cu::bail!("conflicting config detected!");
                    }
                }
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            globs,
        })
    }

    /// Merge another config into self, error if conflicts are detected
    pub fn absorb(&mut self, other: Self) -> cu::Result<()> {
        for (glob, (holder, license)) in other.globs {
            use std::collections::btree_map::Entry;
            match self.globs.entry(glob) {
                Entry::Vacant(entry) => {
                    entry.insert((holder, license));
                }
                Entry::Occupied(entry) => {
                    let glob = entry.key();
                    let (curr_holder, curr_license) = entry.get();
                    if *curr_holder == holder && curr_license.deref() == license.deref() {
                        cu::warn!("glob '{glob}' specified multiple times in multiple configs!");
                        continue;
                    }
                    cu::error!(
                        "conflicting config specified for glob '{glob}' in multiple configs:"
                    );
                    cu::error!("- in one config, it has holder '{holder}' and license '{license}'");
                    cu::error!(
                        "- in another, it has holder '{curr_holder}' and license '{curr_license}'"
                    );
                    cu::bail!("conflicting config detected!");
                }
            }
        }
        Ok(())
    }
}

impl IntoIterator for Config {
    type Item = ConfigEntry;
    type IntoIter = ConfigIntoIter;
    fn into_iter(self) -> Self::IntoIter {
        ConfigIntoIter {
            root: Arc::new(self.root),
            globs_iter: self.globs.into_iter(),
        }
    }
}

pub struct ConfigIntoIter {
    root: Arc<PathBuf>,
    globs_iter: std::collections::btree_map::IntoIter<String, (Arc<String>, Arc<String>)>,
}

impl Iterator for ConfigIntoIter {
    type Item = ConfigEntry;
    fn next(&mut self) -> Option<Self::Item> {
        let (glob, (holder, license)) = self.globs_iter.next()?;
        Some(ConfigEntry {
            root: Arc::clone(&self.root),
            glob,
            holder,
            license,
        })
    }
}

/// Config for one file
pub struct ConfigEntry {
    pub root: Arc<PathBuf>,
    pub glob: String,
    pub holder: Arc<String>,
    pub license: Arc<String>,
}
