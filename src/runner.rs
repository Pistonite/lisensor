// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Pistonite

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::format;

pub async fn run<I>(iter: I, fix: bool) -> cu::Result<()>
where
    I: IntoIterator<Item = (String, Arc<String>, Arc<String>)>,
{
    let bar = cu::progress_unbounded_lowp(if fix {
        "fixing files"
    } else {
        "processing files"
    });

    let mut path_map = BTreeMap::new();
    let mut handles = Vec::new();
    let mut no_match_glob = Vec::new();
    let mut glob_errors = Vec::new();

    // avoid opening too many files. max open 1024 files
    let pool = cu::co::pool(1024);
    for (glob, holder, license) in iter.into_iter() {
        let result = run_glob(
            &glob,
            holder,
            license,
            fix,
            &pool,
            &mut handles,
            &mut path_map,
        );
        match result {
            Ok(matched) => {
                if !matched {
                    no_match_glob.push(glob);
                }
            }
            Err(e) => {
                glob_errors.push((glob, e));
            }
        }
    }
    // put handles into a set to be auto aborted
    // with error handling below
    let total = handles.len();
    bar.set_total(total);
    let mut set = cu::co::set(handles);

    // handle glob errors first
    if !glob_errors.is_empty() {
        for (glob, error) in &glob_errors {
            cu::error!("while globbing '{glob}': {error}");
        }
        cu::error!(
            "got {} errors while searching for files, see above",
            glob_errors.len()
        );
        cu::bail!("error while searching for files");
    }

    let mut count = 0;
    let mut failed = 0;
    while let Some(result) = set.next().await {
        // join error
        let (path, ok) = result?;
        // handle check error
        if !ok {
            failed += 1;
        }
        count += 1;
        cu::progress!(&bar, count, "{}", path.display());
    }

    if failed != 0 {
        cu::error!("checked {total} files, found {failed} issue(s).");
        cu::hint!("run with --fix to fix them automatically.");
        if fix {
            cu::bail!("some issues could not be fixed automatically.");
        } else {
            cu::bail!("license check unsuccesful.");
        }
    }

    cu::info!("license check successful for {total} files.");
    Ok(())
}

fn run_glob(
    glob: &str,
    holder: Arc<String>,
    license: Arc<String>,
    fix: bool,
    pool: &cu::co::Pool,
    handles: &mut Vec<cu::co::Handle<(PathBuf, bool)>>,
    path_map: &mut BTreeMap<PathBuf, (Arc<String>, Arc<String>)>,
) -> cu::Result<bool> {
    let mut matched = false;
    for path in cu::fs::glob(glob)? {
        matched = true;
        let path = path?;
        let holder = Arc::clone(&holder);
        let license = Arc::clone(&license);

        // in fix mode, run additional check for if there are conflicts
        // in the config. Otherwise, the fix result is arbitrary
        let handle = if fix {
            use std::collections::btree_map::Entry;
            match path_map.entry(path.clone()) {
                Entry::Occupied(e) => {
                    let (existing_h, existing_l) = e.get();
                    if (existing_h, existing_l) != (&holder, &license) {
                        cu::error!(
                            "file '{}' matched by multiple globs of conflicting config!",
                            e.key().display()
                        );
                        cu::error!(
                            "- in one config, it has holder '{holder}' and license '{license}'"
                        );
                        cu::error!(
                            "- in another, it has holder '{existing_h}' and license '{existing_l}'"
                        );
                        cu::bail!(
                            "conflicting config found for '{}', while globbing '{glob}'",
                            e.key().display()
                        );
                    }
                    // since the file is already checked by previous job,
                    // we can just skip it
                    continue;
                }
                Entry::Vacant(e) => e.insert((Arc::clone(&holder), Arc::clone(&license))),
            };
            pool.spawn(async move {
                let check_result = format::check_file(&path, &holder, &license);
                let Err(e) = check_result else {
                    return (path, true);
                };
                cu::trace!("'{}': {e}", path.display());
                cu::debug!("fixing '{}'", path.display());
                let Err(e) = format::fix_file(&path, &holder, &license) else {
                    return (path, true);
                };
                cu::error!("failed to fix '{}': {e}", path.display());
                (path, false)
            })
        } else {
            pool.spawn(async move {
                let Err(e) = format::check_file(&path, &holder, &license) else {
                    return (path, true);
                };
                cu::warn!("'{}': {e}", path.display());
                (path, false)
            })
        };

        handles.push(handle);
    }

    Ok(matched)
}
