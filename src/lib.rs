// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

//! See README on crates.io or GitHub.

mod config;
pub use config::*;

mod runner;
pub use runner::*;
mod format;
pub use format::*;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
pub use cli::*;
