// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use cu::pre::*;

mod config;
use config::*;
mod format;
mod runner;

/// Check or fix license notices
#[derive(clap::Parser, Clone)]
struct Cli {
    /// Attempt fix the license notice on the files
    #[clap(short, long)]
    fix: bool,
    /// In inline config mode, specify the copyright holder
    #[clap(short = 'H', long, requires("license"))]
    holder: Option<String>,
    /// In inline config mode, specify the SPDX ID for the license
    #[clap(short = 'L', long, requires("holder"))]
    license: Option<String>,

    #[clap(flatten)]
    common: cu::cli::Flags,

    /// Paths to config files, or in inline config mode, glob patterns for source files
    /// to apply the license notice.
    paths: Vec<String>,
}

#[cu::cli(flags = "common")]
async fn main(mut args: Cli) -> cu::Result<()> {
    let config = load_config(&mut args)?;
    runner::run(config.into_iter(), args.fix).await
}
