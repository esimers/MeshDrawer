use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};
use crate::paths::{normalize_abs, to_rel_slash};

const INVALID: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "lpt1", "lpt2", "lpt3",
];

pub fn next_path(old: &Path, input: &str) -> Result<PathBuf> {
    let ext = old
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| AppError::Message("file has no extension".into()))?;
    let stem = sanitized_stem(input, ext)?;
    let parent = old
        .parent()
        .ok_or_else(|| AppError::Message("file has no folder".into()))?;
    Ok(parent.join(format!("{stem}.{ext}")))
}

pub fn sanitized_stem(input: &str, original_ext: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AppError::Message("name cannot be empty".into()));
    }
    if trimmed.chars().any(|c| INVALID.contains(&c) || c.is_control()) {
        return Err(AppError::Message("name contains invalid characters".into()));
    }
    let mut stem = trimmed.to_string();
    let dotted = format!(".{original_ext}");
    if stem.to_ascii_lowercase().ends_with(&dotted.to_ascii_lowercase()) {
        stem.truncate(stem.len() - dotted.len());
    }
    let stem = stem.trim().trim_end_matches('.');
    if stem.is_empty() {
        return Err(AppError::Message("name cannot be empty".into()));
    }
    if RESERVED.contains(&stem.to_ascii_lowercase().as_str()) {
        return Err(AppError::Message("that name is reserved on Windows".into()));
    }
    Ok(stem.to_string())
}

pub fn rename_on_disk(old: &Path, input: &str) -> Result<PathBuf> {
    let dest = next_path(old, input)?;
    if dest == old {
        return Ok(normalize_abs(old));
    }
    if dest.exists() {
        return Err(AppError::path("a file with that name already exists", dest));
    }
    fs::rename(old, &dest)?;
    Ok(normalize_abs(&dest))
}

pub fn rel_path_for(root: &Path, abs: &Path) -> String {
    match abs.strip_prefix(root) {
        Ok(rel) => to_rel_slash(rel),
        Err(_) => abs
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn keeps_original_extension() {
        let old = Path::new(r"C:\lib\dragon.STL");
        let dest = next_path(old, "wing").unwrap();
        assert_eq!(dest.file_name().unwrap(), "wing.STL");
        let dest = next_path(old, "wing.stl").unwrap();
        assert_eq!(dest.file_name().unwrap(), "wing.STL");
    }

    #[test]
    fn rejects_bad_names() {
        assert!(sanitized_stem("", "stl").is_err());
        assert!(sanitized_stem("a/b", "stl").is_err());
        assert!(sanitized_stem("con", "stl").is_err());
        assert!(sanitized_stem("ok name", "stl").unwrap() == "ok name");
    }

    #[test]
    fn rename_roundtrip() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("a.stl");
        File::create(&src).unwrap();
        let dest = rename_on_disk(&src, "b").unwrap();
        assert!(dest.ends_with("b.stl"));
        assert!(dest.exists());
        assert!(!src.exists());
    }
}
