// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use cu::pre::*;

/// Convert the CLI args into configuration object
pub fn load_config(args: &mut crate::Cli) -> cu::Result<Config> {
    match (args.holder.take(), args.license.take()) {
        (Some(holder), Some(license)) => {
            if let Some(config_path) = find_config_path() {
                cu::bail!(
                    "--holder or --license cannot be specified when {config_path} is present in the current directory"
                );
            }
            Ok(Config::inline(
                holder,
                license,
                std::mem::take(&mut args.paths),
            ))
        }
        // clap ensures both are None
        _ => {
            let mut iter = std::mem::take(&mut args.paths).into_iter();
            let mut config = match iter.next() {
                None => {
                    let Some(config_path) = find_config_path() else {
                        cu::bail!(
                            "cannot find Lisensor.toml, and no config files are specified on the command line."
                        );
                    };
                    Config::build(config_path)?
                }
                Some(first) => Config::build(&first)?,
            };

            for path in iter {
                config.absorb(Config::build(&path)?)?;
            }

            Ok(config)
        }
    }
}

fn find_config_path() -> Option<&'static str> {
    ["Lisensor.toml", "lisensor.toml"]
        .into_iter()
        .find(|x| Path::new(x).exists())
}

/// Raw config read from a toml config file.
///
/// The format is holder -> glob -> license
#[derive(Deserialize)]
struct TomlConfig(BTreeMap<String, BTreeMap<String, String>>);

/// Config object
pub struct Config {
    // glob -> (holder, license)
    globs: BTreeMap<String, (Arc<String>, Arc<String>)>,
}
impl Config {
    pub fn inline(holder: String, license: String, glob_list: Vec<String>) -> Self {
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
    /// Build the config from a path, error if conflicts are detected
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
    pub fn into_iter(self) -> impl Iterator<Item = (String, Arc<String>, Arc<String>)> {
        self.globs
            .into_iter()
            .map(|(path, (holder, license))| (path, holder, license))
    }
}
