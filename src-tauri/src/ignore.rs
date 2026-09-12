use std::path::Path;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".meshdrewer-quarantine",
    "$recycle.bin",
    "system volume information",
];

const SKIP_FILE_SUFFIXES: &[&str] = &[".tmp", ".partial", ".download"];

pub fn is_model_ext(ext: &str) -> bool {
    matches!(ext, "stl" | "3mf" | "obj")
}

pub fn extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
}

pub fn companion_kind(ext: &str) -> Option<&'static str> {
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => Some("image"),
        "pdf" => Some("pdf"),
        "txt" | "md" => Some("txt"),
        _ => None,
    }
}

pub fn skip_dir_name(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    SKIP_DIRS.iter().any(|d| *d == lower)
}

pub fn skip_file_name(name: &str) -> bool {
    if name.starts_with('.') || name.starts_with("~$") {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    SKIP_FILE_SUFFIXES.iter().any(|s| lower.ends_with(s))
}

pub fn matches_exclude(path: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let hay = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    for raw in patterns {
        let pat = raw.trim().trim_start_matches('/').replace('\\', "/").to_ascii_lowercase();
        if pat.is_empty() || pat.starts_with('#') {
            continue;
        }
        if pat.contains('*') {
            let needle = pat.trim_matches('*');
            if !needle.is_empty() && (hay.contains(needle) || name.contains(needle)) {
                return true;
            }
        } else if hay.contains(&pat) || name == pat {
            return true;
        }
    }
    false
}

pub fn skip_path(path: &Path, is_dir: bool) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if is_dir {
            return skip_dir_name(name);
        }
        return skip_file_name(name);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn model_exts_are_case_folded_by_caller() {
        assert!(is_model_ext("stl"));
        assert!(is_model_ext("3mf"));
        assert!(is_model_ext("obj"));
        assert!(!is_model_ext("step"));
    }

    #[test]
    fn skips_hidden_and_tool_dirs() {
        assert!(skip_dir_name(".git"));
        assert!(skip_dir_name("node_modules"));
        assert!(skip_dir_name(".meshdrewer-quarantine"));
        assert!(!skip_dir_name("Models"));
    }

    #[test]
    fn skips_temp_and_partial_files() {
        assert!(skip_file_name("foo.tmp"));
        assert!(skip_file_name("model.3mf.partial"));
        assert!(skip_file_name("~$lock"));
        assert!(!skip_file_name("dragon.stl"));
        assert!(skip_path(Path::new("foo.download"), false));
    }

    #[test]
    fn exclude_patterns_match_path_or_name() {
        let path = Path::new(r"C:\lib\old\dragon.stl");
        assert!(matches_exclude(path, &["old".into()]));
        assert!(matches_exclude(path, &["*.bak".into(), "*dragon*".into()]));
        assert!(!matches_exclude(path, &["*.bak".into()]));
    }
}
