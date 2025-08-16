// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use cu::pre::*;

/// Try finding the default config files according to the order
/// specified in the documentation (see repo README)
pub fn try_find_default_config_file() -> Option<&'static str> {
    ["Lisensor.toml", "lisensor.toml"]
        .into_iter()
        .find(|x| Path::new(x).exists())
}

/// Config object
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    // glob -> (holder, license)
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
    pub fn new(holder: String, license: String, glob_list: Vec<String>) -> Self {
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
        Self { globs }
    }

    /// Build the config by reading the file specified, error if conflicts are detected
    ///
    /// The globs specified in the config file are relative to the parent directory
    /// of `path`.
    pub fn build(path: &str) -> cu::Result<Self> {
        let raw = toml::parse::<TomlConfig>(&cu::fs::read_string(path)?)?;
        let parent = Path::new(path)
            .parent()
            .context("failed to get parent path for config")?;
        let mut globs = BTreeMap::new();
        for (holder, table) in raw.0 {
            let holder = Arc::new(holder);
            for (glob, license) in table {
                // globs in config files are resolved relative
                // to the directory where the config file is in
                let glob = parent.join(glob).into_utf8()?;
                use std::collections::btree_map::Entry;
                match globs.entry(glob) {
                    Entry::Vacant(entry) => {
                        entry.insert((Arc::clone(&holder), Arc::new(license)));
                    }
                    Entry::Occupied(entry) => {
                        let glob = entry.key();
                        let (curr_holder, curr_license) = entry.get();
                        if *curr_holder == holder && curr_license.deref() == license.as_str() {
                            cu::warn!("glob '{glob}' specified multiple times in '{path}'!");
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
        Ok(Self { globs })
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

impl Config {
    /// Iterate the resolve paths as (path, holder, license)
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl Iterator<Item = (String, Arc<String>, Arc<String>)> {
        // we can't implement the IntoIterator trait because
        // the map object has an unnamed function type
        self.globs
            .into_iter()
            .map(|(path, (holder, license))| (path, holder, license))
    }
}
