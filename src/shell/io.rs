//! The only module allowed to touch the outside world: files, stdout, stderr.
//! Everything else in the crate is a pure function of its arguments.

use std::fs;
use std::io::Write as _;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IoError {
    pub path: String,
    pub detail: String,
}

impl IoError {
    pub fn message(&self) -> String {
        format!("'{}': {}", self.path, self.detail)
    }
}

pub fn read_text(path: &Path) -> Result<String, IoError> {
    fs::read_to_string(path).map_err(|error| IoError {
        path: path.display().to_string(),
        detail: error.to_string(),
    })
}

/// Write a generated file, flushing and fsyncing before returning.
pub fn write_text(path: &Path, contents: &str) -> Result<(), IoError> {
    let mut file = fs::File::create(path).map_err(|error| IoError {
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    file.write_all(contents.as_bytes()).map_err(|error| IoError {
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    file.sync_all().map_err(|error| IoError {
        path: path.display().to_string(),
        detail: error.to_string(),
    })
}

/// Write exact bytes (traces already end in a newline).
pub fn write_stdout(text: &str) {
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}

pub fn out(line: &str) {
    println!("{}", line);
}

pub fn err(line: &str) {
    eprintln!("{}", line);
}
