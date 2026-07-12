// SPDX-License-Identifier: MIT
// Copyright (c) 2025-2026 Pistonite

use std::path::Path;

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

    /// Do not attempt discovering config file
    #[clap(long, requires("holder"))]
    pub no_config: bool,

    #[clap(flatten)]
    pub common: cu::cli::Flags,

    #[clap(short = 'V', long)]
    pub version: bool,

    /// Paths to config files, or in inline config mode, glob patterns for source files
    /// to apply the license notice.
    ///
    /// In inline config mode, the glob patterns are relative to the current working directory
    pub paths: Vec<String>,
}

/// Convert the CLI args into configuration object
pub fn config_from_cli(args: &mut crate::Cli) -> cu::Result<Config> {
    match (args.holder.take(), args.license.take()) {
        (Some(holder), Some(license)) => {
            if !args.no_config {
                if let Some(config_path) = crate::try_find_default_config_file() {
                    cu::error!(
                        "--holder or --license cannot be specified when config file is present: '{}'",
                        config_path.display()
                    );
                    cu::hint!(
                        "this error exists to prevent accidental misuse; use --no-config to skip discovering config file if intended"
                    );
                    cu::bail!("invalid command line usage");
                }
            }
            Ok(Config::new(
                std::env::current_dir()?,
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
                            "cannot find config file, and no config files are specified on the command line."
                        );
                    };
                    Config::build(&config_path)?
                }
                Some(first) => Config::build(Path::new(&first))?,
            };

            for path in iter {
                config.absorb(Config::build(Path::new(&path))?)?;
            }

            Ok(config)
        }
    }
}
