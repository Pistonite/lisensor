// SPDX-License-Identifier: MIT
// Copyright (c) 2025-2026 Pistonite

use std::io::BufRead;
use std::path::Path;
use std::sync::LazyLock;

use cu::pre::*;

const DEFAULT_YEAR: u32 = 2025; // The year this tool is made

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// The `// ...` format
    SlashSlash,
    /// The `# ...` format
    Hash,
}

static HASH_FORMAT_EXTENSIONS: &[&str] = &[
    "bash", "ini", "mk", "php", "phtml", "pl", "pm", "ps1", "psd1", "psm1", "py", "r", "rb", "sh",
    "tcl", "toml", "yaml", "yml", "zsh",
];

impl Format {
    pub fn from_path(path: &Path) -> Self {
        let Some(ext) = path.extension().and_then(|x| x.to_str()) else {
            return Self::SlashSlash;
        };

        if HASH_FORMAT_EXTENSIONS.binary_search(&ext).is_ok() {
            return Self::Hash;
        }
        Self::SlashSlash
    }

    /// Strip the license line if it's the right format.
    /// Return the SPDX id
    pub fn check_strip_license_line(self, line: &str) -> Option<&str> {
        match self {
            Self::SlashSlash => line.strip_prefix("// SPDX-License-Identifier: "),
            Self::Hash => line.strip_prefix("# SPDX-License-Identifier: "),
        }
    }

    /// Strip the copyright line if it's the right format.
    /// Return "YYYY[-YYYY] HOLDER"
    pub fn check_strip_copyright_line(self, line: &str) -> Option<&str> {
        match self {
            Self::SlashSlash => line.strip_prefix("// Copyright (c) "),
            Self::Hash => line.strip_prefix("# Copyright (c) "),
        }
    }

    /// Check if the line starts with sentinel comment
    pub fn starts_with_sentinel(self, line: &str) -> bool {
        match self {
            Self::SlashSlash => line.starts_with("// * * * * *"),
            Self::Hash => line.starts_with("# * * * * *"),
        }
    }

    /// Format the license notice into a buffer
    pub fn format(
        self,
        year_start: u32,
        holder: &str,
        license: &str,
        is_crlf: bool,
        buf: &mut String,
    ) -> cu::Result<()> {
        use std::fmt::Write as _;
        let year_end = current_year();
        let le = if is_crlf { "\r\n" } else { "\n" };
        match self {
            Self::SlashSlash => {
                if year_start == year_end {
                    write!(
                        buf,
                        "// SPDX-License-Identifier: {license}{le}// Copyright (c) {year_start} {holder}{le}"
                    )?;
                } else {
                    write!(
                        buf,
                        "// SPDX-License-Identifier: {license}{le}// Copyright (c) {year_start}-{year_end} {holder}{le}"
                    )?;
                }
            }
            Self::Hash => {
                if year_start == year_end {
                    write!(
                        buf,
                        "# SPDX-License-Identifier: {license}{le}# Copyright (c) {year_start} {holder}{le}"
                    )?;
                } else {
                    write!(
                        buf,
                        "# SPDX-License-Identifier: {license}{le}# Copyright (c) {year_start}-{year_end} {holder}{le}"
                    )?;
                }
            }
        }
        Ok(())
    }
}

pub fn check_file(path: &Path, expected_holder: &str, expected_license: &str) -> cu::Result<()> {
    let format = Format::from_path(path);
    let reader = cu::fs::reader(path)?;
    let mut lines = reader.lines();

    let line = cu::check!(lines.next(), "missing license notice line.")?;
    let line = cu::check!(line, "error while reading file '{}'", path.display())?;

    let actual_license = format.check_strip_license_line(&line);
    let actual_license = cu::check!(actual_license, "missing license notice line.")?;
    if actual_license != expected_license {
        cu::bail!("license is wrong: expected '{expected_license}', found '{actual_license}'.");
    }

    let line = cu::check!(lines.next(), "missing copyright line.")?;
    let line = cu::check!(line, "error while reading file '{}'", path.display())?;

    let copyright_info = format.check_strip_copyright_line(&line);
    let copyright_info = cu::check!(copyright_info, "missing copyright line.")?;

    let (_, year_end, actual_holder) = parse_copyright_info(copyright_info);
    if actual_holder != expected_holder {
        cu::bail!("holder is wrong: expected '{expected_holder}', found '{actual_holder}'.");
    }
    let current_year = current_year();
    if year_end != current_year {
        cu::bail!("copyright info ends at {year_end}, but we are in {current_year}.");
    }

    Ok(())
}

pub fn fix_file(path: &Path, expected_holder: &str, expected_license: &str) -> cu::Result<()> {
    let format = Format::from_path(path);
    let file_content = cu::fs::read_string(path)?;
    let lines = file_content.lines();
    let mut buf = FixBuf::default();
    // usually this should only go through the first line
    // unless the file is unconventional
    if file_content.contains("\r\n") {
        cu::debug!("will use CRLF for file '{}'", path.display());
        buf.set_crlf(true);
    }

    let mut found_license_line = false;
    let mut found_copyright_line = false;
    let mut found_sentinel = false;

    for line in lines {
        if found_sentinel {
            buf.push_line(line, format);
            continue;
        }
        if format.starts_with_sentinel(line) {
            found_sentinel = true;
            buf.push_line(line, format);
            continue;
        }
        if format.check_strip_license_line(line).is_some() {
            if found_license_line {
                cu::bail!(
                    "multiple license line found! Consider adding a sentinel line if there are other license notices that need to be kept!"
                );
            }
            found_license_line = true;
            continue;
        }
        if let Some(copyright_info) = format.check_strip_copyright_line(line) {
            if found_copyright_line {
                cu::bail!(
                    "multiple copyright line found! Consider adding a sentinel line if there are other license notices that need to be kept!"
                );
            }
            found_copyright_line = true;
            let (year_start, _, _) = parse_copyright_info(copyright_info);
            if year_start > current_year() {
                cu::bail!("copyright start year is in the future! Manual fix required.");
            }
            buf.perform_fix_if_need(format, year_start, expected_holder, expected_license)?;
            continue;
        }
        buf.push_line(line, format);
    }
    // format new notice if didn't find one
    buf.perform_fix_if_need(format, current_year(), expected_holder, expected_license)?;

    cu::fs::write(path, buf.buf)?;
    Ok(())
}

#[derive(Default)]
struct FixBuf {
    buf: String,
    is_crlf: bool,
    fixed: bool,
    fixed_when_empty: bool,
}
impl FixBuf {
    pub fn set_crlf(&mut self, crlf: bool) {
        self.is_crlf = crlf;
    }
    fn push_line(&mut self, line: &str, format: Format) {
        let le_byte_len = if self.is_crlf { 2 } else { 1 };
        if self.fixed_when_empty {
            if !format.starts_with_sentinel(line) && !line.is_empty() {
                self.buf.reserve(line.len() + le_byte_len * 2);
                self.push_line_ending();
            } else {
                self.buf.reserve(line.len() + le_byte_len);
            }
            self.fixed_when_empty = false;
        } else {
            self.buf.reserve(line.len() + le_byte_len);
        }

        self.buf.push_str(line);
        self.push_line_ending();
    }
    fn perform_fix_if_need(
        &mut self,
        format: Format,
        year_start: u32,
        holder: &str,
        license: &str,
    ) -> cu::Result<()> {
        if self.fixed {
            return Ok(());
        }
        let current_content = std::mem::take(&mut self.buf);
        format.format(year_start, holder, license, self.is_crlf, &mut self.buf)?;
        // add an empty line if needed
        if !current_content.is_empty() {
            if !format.starts_with_sentinel(&current_content) && !current_content.starts_with('\n')
            {
                self.push_line_ending();
            }
        } else {
            self.fixed_when_empty = true;
        }
        self.buf.push_str(&current_content);
        self.fixed = true;
        Ok(())
    }
    fn push_line_ending(&mut self) {
        if self.is_crlf {
            self.buf.push_str("\r\n");
        } else {
            self.buf.push('\n');
        }
    }
}

fn parse_copyright_info(info: &str) -> (u32, u32, &str) {
    let mut parts = info.splitn(2, ' ');
    let (year_start, year_end) = match parts.next() {
        None => (DEFAULT_YEAR, DEFAULT_YEAR),
        Some(x) => {
            let mut parts = x.splitn(2, '-');
            let year_start = parts
                .next()
                .and_then(|x| cu::parse::<u32>(x).ok())
                .unwrap_or(DEFAULT_YEAR);
            let year_end = parts
                .next()
                .and_then(|x| cu::parse::<u32>(x).ok())
                .unwrap_or(year_start);
            (year_start, year_end.max(year_start))
        }
    };
    let holder = parts.next().unwrap_or("");
    (year_start, year_end, holder)
}

fn current_year() -> u32 {
    if cfg!(test) {
        // mock current year in tests
        return DEFAULT_YEAR;
    }
    static YEAR: LazyLock<u32> = LazyLock::new(|| {
        use chrono::Datelike;
        let y = chrono::Local::now().year().max(0) as u32;
        cu::debug!("current year is {y}");
        y
    });
    *YEAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_format_extensions() {
        let mut x = HASH_FORMAT_EXTENSIONS.to_vec();
        x.sort();
        assert_eq!(
            x, HASH_FORMAT_EXTENSIONS,
            "HASH_FORMAT_EXTENSIONS must be sorted"
        );
    }
}
