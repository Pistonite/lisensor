# lisensor

![Build Badge](https://img.shields.io/github/check-runs/Pistonite/lisensor/main)
![License Badge](https://img.shields.io/github/license/Pistonite/lisensor)
![Issue Badge](https://img.shields.io/github/issues/Pistonite/lisensor)

Lisensor (pronounced *licenser*) is a tool that automatically adds a
license notice to your source file, like this:

```
// SPDX-License-Identifier: [License]
// Copyright (c) [Year] [Holder]
```

See https://spdx.dev/learn/handling-license-info/ for more information
about this format.

For languages such as python, the comment style will automatically
be changed to `#` instead of `//`. Languages that do not have
either of the comment styles are currently not supported. (Feel free to PR).

The CLI usage is:
```
lisensor [CONFIG] ... [-f]
```

`CONFIG` is one of more config file for Lisensor (see below). `-f` will attempt to automatically
fix the files in place.

## Config
By default, `lisensor` looks for `Lisensor.toml` then `lisensor.toml`
in the current directory if no config files are specified.
Paths in the config file are relative to the directory containing
the config file, meaning running `lisensor` from anywhere will resulting
in the same outcome.

The config file should contain one table per copyright holder.
The table should contain key-value pairs, where the keys are
globs (absolute or relative to the directory containing the config file),
and the value is any SPDX ID, but the value is not validated.

For example:

```toml
[Pistonite]
"**/*.rs" = "MIT"
```

## Inline Config
When the config is small, you can specify it directly in the CLI using
the following flags. You cannot use these flags if a config file is specified,
or a compatible config is found in the current directory.

```
lisensor --holder HOLDER --license LICENSE [GLOB_PATTERN] ...
```

- `--holder HOLDER`: specify the copyright holder
- `--license LICENSE`: specify the SPDX ID
- `GLOB_PATTERN`: specify one or more glob patterns, absolute or resolved
  from the current directory.

You can only specify one holder and one license type using inline config mode.

## Conflict Resolution
If a glob pattern is specified multiple times with a different config,
that will be caught and will be reported.

If a file is covered by multiple glob patterns with conflicting configs
(such as different copyright holder or different license), check (running without `-f`)
will definitely fail, because it's not possible for one file
to satisfy multiple conflicting configs. However, it will not be presented
as a conflict, but as a regular license/holder mismatch.
The fix mode (running with `-f`) has an extra check to report an error
if a file is covered by conflicting configs. This prevents "fake" successful
fixes. However, the fix mode might still edit the file according to one
of the configs specified (arbitrarily chosen) before reporting the error.

## Line Ending
When checking, any line ending is accepted. When fixing, it will turn the file
into UNIX line ending.

When checking, only the license and copyright lines are checked.
When fixing, it will add an empty line between the notice
and the rest of the file, if there isn't already.
