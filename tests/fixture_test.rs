// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use std::path::Path;

use lisensor::{Config, run};

pub fn run_fixture(name: &str) -> cu::Result<()> {
    cu::init_print_options(cu::lv::Color::Never, cu::lv::Print::QuietQuiet, None);
    let update_output = std::env::var("FIXTURE_UPDATE").unwrap_or_default().as_str() == "1";

    let fixtures = Path::new("tests").join("fixtures");
    let input_path = fixtures.join(name);

    cu::debug!("running fixture: {name}");

    let input_copy_path = fixtures.join(format!("{name}_out"));
    std::fs::copy(&input_path, &input_copy_path)?;

    let config = Config::new(
        "TestHolder".to_string(),
        "TestLicense".to_string(),
        vec![input_copy_path.to_string_lossy().into_owned()],
    );
    let config2 = config.clone();

    let check_result = cu::co::run(async move { run(config, false).await })?;
    let expected_failure = fixtures.join(format!("{name}_cfail"));
    if expected_failure.exists() {
        let expected_error = cu::fs::read_string(&expected_failure)?;
        match check_result {
            Ok(_) => {
                if !expected_error.trim().is_empty() {
                    cu::bail!("fixture {name} is supposed to fail in check mode, but it passed.");
                }
            }
            Err(e) => {
                let actual_error = e.to_string();
                if actual_error != expected_error {
                    if update_output {
                        cu::fs::write(expected_failure, actual_error)?;
                    } else {
                        cu::bail!(
                            "fixture '{name}' check error mismatch.\nexpected={expected_error}\nactual={actual_error}"
                        );
                    }
                }
            }
        }
    } else {
        if let Err(e) = check_result {
            cu::fs::write(expected_failure, e.to_string())?;
        } else {
            cu::fs::write(expected_failure, "")?;
        }
    }

    let fix_result = cu::co::run(async move { run(config2, true).await })?;

    let expected_failure = fixtures.join(format!("{name}_ffail"));
    if expected_failure.exists() {
        let expected_error = cu::fs::read_string(&expected_failure)?;
        match fix_result {
            Ok(_) => {
                if !expected_error.trim().is_empty() {
                    cu::bail!("fixture {name} is supposed to fail in fix mode, but it passed.");
                }
            }
            Err(e) => {
                let actual_error = e.to_string();
                if actual_error != expected_error {
                    if update_output {
                        cu::fs::write(expected_failure, actual_error)?;
                    } else {
                        cu::bail!(
                            "fixture '{name}' fix error mismatch.\nexpected={expected_error}\nactual={actual_error}"
                        );
                    }
                }
            }
        }
    } else {
        if let Err(e) = fix_result {
            cu::fs::write(expected_failure, e.to_string())?;
        } else {
            cu::fs::write(expected_failure, "")?;
        }
    }

    let expected_output = fixtures.join(format!("{name}_fixed"));
    if expected_output.exists() {
        let expected_output_content = cu::fs::read_string(&expected_output)?;
        let actual_output_content = cu::fs::read_string(input_copy_path)?;
        if expected_output_content != actual_output_content {
            if update_output {
                cu::fs::write(expected_output, actual_output_content)?;
            } else {
                cu::bail!("fixture '{name}' output mismatch. actual:\n{actual_output_content}");
            }
        }
    } else {
        std::fs::copy(input_copy_path, expected_output)?;
    }

    Ok(())
}

macro_rules! run_fixture {
    ($name:ident) => {
        #[test]
        fn $name() -> cu::Result<()> {
            run_fixture(concat!(stringify!($name), ".txt"))
        }
    };
}

run_fixture!(empty_text);
run_fixture!(only_1line);
run_fixture!(wrong_license);
run_fixture!(wrong_holder);
run_fixture!(wrong_holder_license);
run_fixture!(after_copyright);
run_fixture!(before_license);
run_fixture!(middle_license);
run_fixture!(multi_correct);
run_fixture!(multi_wrong);
run_fixture!(sentinel_first);
run_fixture!(wrong_year);
run_fixture!(wrong_year_future);
run_fixture!(wrong_year_future_range);
run_fixture!(wrong_year_range);
