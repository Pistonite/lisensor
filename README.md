# lisensor

![Build Badge](https://img.shields.io/github/check-runs/Pistonite/lisensor/main)
![License Badge](https://img.shields.io/github/license/Pistonite/lisensor)
![Issue Badge](https://img.shields.io/github/issues/Pistonite/lisensor)

```
# install from source
cargo install lisensor
# install prebuilt binary with cargo-binstall
cargo binstall lisensor
```

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

## Usage

The CLI usage is:
```
lisensor [CONFIG] ... [-f]
```

`CONFIG` is one of more config file for Lisensor (see below). `-f` will attempt to automatically
fix the files in place.

The crate also has a library target of the same name. It's intended to be
used by tests in the project, but might be helpful for integrating `lisensor` into your own tooling.

## Config
By default, `lisensor` looks for `Lisensor.toml` then `lisensor.toml`
in the current directory if no config files are specified.

Globs in the config file are relative to the directory containing
the config file, meaning running `lisensor` from anywhere will result
in the same outcome. Currently, symlinks are not followed.

The config file should contain one table per copyright holder.
The table should contain key-value pairs, where the keys are
globs (absolute or relative to the directory containing the config file),
and the value is any SPDX ID, but the value is not validated.

For example:

```toml
["Foobar contributors"]
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

## Compatibility with Other License Notices
It's common if some file is taken from another project, you must include
a license notice if it's not already in the file. In this case,
it's recommended to use a sentinel line (see Format Behavior Details below)
to prevent the tool from accidentally overriding the original notice.

```
// See original license notices below:
// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Other people
// 
// ^ This is bad because running the tool will now
//   change the holder and license into you, even though what you want
//   is to add another notice on top of it
```

Adding a sentinel line will fix this:
```
// See original license notices below:
// * * * * *
// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Other people
// 
// ^ Now the tool will ignore everything after * * * * *
```

## Format Behavior Details
The tool will check and ensure the following conditions are true for a file:

1. The first line is the license line, with the comment style for that file,
   and starting with `SPDX-License-Identifier: ` after the comment, followed
   by the expected license identifier.
2. The second line is the copy right line, with the comment style for that file,
   and starting with `Copyright (c) `, follow by a year range, where the start
   can be anything and the end must be the current year at the local time the tool
   is ran. If the start and end are the same year, then a single year is sufficient.
   The year range is followed by a space, then the copyright holder.
3. No other line(s) exist that matches the same `SPDX-License-Identifier`
   or `Copyright (c)` format for license and copyright lines, respectively.

The tool will not attempt fixing the file, if any copyright line is found
with the wrong holder. This ensures that the tool never accidentally override
license notices from the original source file.

If a source file contains license notice(s) from its original authors,
you must specify a *sentinel* line after your license notice. The tool
will skip checking all contents after the sentinel line. The sentinel line
is the same comment style followed by `* * * * *` (five `*` separated by four spaces).
Anything can follow after that. For example:

```
// SPDX-License-Identifier: MIT
// Copyright (c) 2024-2025 Foobar contributors
// * * * * * other license information below
// SPDX-License-Identifier: MIT
// Copyright (c) 2017-2018 Bizbaz contributors
```

When the year becomes `2026`, the tool will change the notice
for `Foobar contributors`, but will not touch anything below the sentinel line.

Other:

- Line ending:
  - When checking, any line ending is accepted. When fixing, it will turn the file
    into UNIX line ending.
  - When checking, only the first 2 lines are checked, the rest of the file
    is ignored.
  - When fixing, if the third line is not a sentinel line or empty line,
    it will ensure there's an empty line between the license notice and the
    rest of the content.
