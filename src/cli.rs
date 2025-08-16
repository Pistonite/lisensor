// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use cu::pre::*;

use crate::Config;

/// Check or fix license notices
#[derive(Debug, Clone, PartialEq, clap::Parser)]
pub struct Cli {
    /// Attempt fix the license notice on the files
    #[clap(short, long)]
    pub fix: bool,
    /// In inline config mode, specify the copyright holder
    #[clap(short = 'H', long, requires("license"))]
    pub holder: Option<String>,
    /// In inline config mode, specify the SPDX ID for the license
    #[clap(short = 'L', long, requires("holder"))]
    pub license: Option<String>,

    #[clap(flatten)]
    pub common: cu::cli::Flags,

    /// Paths to config files, or in inline config mode, glob patterns for source files
    /// to apply the license notice.
    pub paths: Vec<String>,
}

/// Convert the CLI args into configuration object
pub fn config_from_cli(args: &mut crate::Cli) -> cu::Result<Config> {
    match (args.holder.take(), args.license.take()) {
        (Some(holder), Some(license)) => {
            if let Some(config_path) = crate::try_find_default_config_file() {
                cu::bail!(
                    "--holder or --license cannot be specified when {config_path} is present in the current directory"
                );
            }
            Ok(Config::new(
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
                    let Some(config_path) = crate::try_find_default_config_file() else {
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
