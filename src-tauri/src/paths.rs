use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};

/// `%LOCALAPPDATA%\MeshDrawer` on Windows; Tauri app-local data dir elsewhere.
pub fn app_data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| AppError::Message("LOCALAPPDATA is not set".into()))?;
        Ok(base.join("MeshDrawer"))
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .ok_or_else(|| AppError::Message("could not resolve local data directory".into()))?;
        Ok(base.join("MeshDrawer"))
    }
}

pub fn db_path() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("library.db"))
}

pub fn thumbs_dir() -> Result<PathBuf> {
    Ok(app_data_dir()?.join("thumbs"))
}

pub fn ensure_app_dirs() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let data = app_data_dir()?;
    let thumbs = thumbs_dir()?;
    std::fs::create_dir_all(&data)?;
    std::fs::create_dir_all(&thumbs)?;
    Ok((data, db_path()?, thumbs))
}

/// Store paths with `/` separators in `rel_path`. Absolute paths stay OS-native.
#[allow(dead_code)]
pub fn to_rel_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn normalize_abs(path: &Path) -> PathBuf {
    dunce::simplified(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_slash_normalizes_backslashes() {
        assert_eq!(to_rel_slash(Path::new(r"models\dragon.stl")), "models/dragon.stl");
    }
}
