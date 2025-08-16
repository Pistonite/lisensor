// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use lisensor::{Cli, config_from_cli, run};

#[cu::cli(flags = "common")]
async fn main(mut args: Cli) -> cu::Result<()> {
    let fix = args.fix;
    let result = run(config_from_cli(&mut args)?, fix).await?;

    if result.is_err() {
        if fix {
            cu::bail!("some issues could not be fixed automatically.");
        } else {
            cu::bail!("license check unsuccesful.");
        }
    }

    Ok(())
}
