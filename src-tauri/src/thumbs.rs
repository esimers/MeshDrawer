use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use zip::ZipArchive;

use crate::db::{self, Db};
use crate::error::{AppError, Result};
use crate::paths::{self, thumbs_dir};

const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbReady {
    pub id: i64,
    pub thumb_path: Option<String>,
    pub thumb_status: String,
    pub plate_count: i64,
}

pub fn cover_name(file_id: i64) -> String {
    format!("{file_id}_cover.png")
}

pub fn cover_abs(file_id: i64) -> Result<PathBuf> {
    Ok(thumbs_dir()?.join(cover_name(file_id)))
}

pub fn extract_3mf_cover(path: &Path) -> Result<Option<(Vec<u8>, i64)>> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            archive
                .by_index(i)
                .ok()
                .map(|entry| normalize_zip_name(entry.name()))
        })
        .collect();

    let plate_count = plate_count_from_archive(&mut archive, &names);
    let cover = cover_png(&mut archive, &names);
    Ok(cover.map(|bytes| (bytes, plate_count.max(1))))
}

fn normalize_zip_name(name: &str) -> String {
    name.replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

fn read_named(archive: &mut ZipArchive<fs::File>, names: &[String], want: &str) -> Option<Vec<u8>> {
    let idx = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(want))?;
    let mut entry = archive.by_index(idx).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    if buf.starts_with(PNG_MAGIC) {
        Some(buf)
    } else {
        None
    }
}

fn plate_count_from_archive(archive: &mut ZipArchive<fs::File>, names: &[String]) -> i64 {
    if let Some(xml) = read_text(archive, names, "Metadata/model_settings.config") {
        let count = xml.matches("<plate").count() as i64;
        if count > 0 {
            return count;
        }
    }
    let mut max_n = 0_i64;
    for name in names {
        let lower = name.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("metadata/plate_") {
            if let Some(num) = rest
                .trim_end_matches(".png")
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|s| s.parse::<i64>().ok())
            {
                max_n = max_n.max(num);
            }
        }
    }
    max_n
}

fn read_text(archive: &mut ZipArchive<fs::File>, names: &[String], want: &str) -> Option<String> {
    let idx = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case(want))?;
    let mut entry = archive.by_index(idx).ok()?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn cover_png(archive: &mut ZipArchive<fs::File>, names: &[String]) -> Option<Vec<u8>> {
    if let Some(xml) = read_text(archive, names, "Metadata/model_settings.config") {
        if let Some(target) = first_thumbnail_file(&xml) {
            if let Some(bytes) = read_named(archive, names, &target) {
                return Some(bytes);
            }
        }
    }

    for candidate in [
        "Metadata/plate_1.png",
        "Metadata/plate_1_small.png",
        "Metadata/thumbnail.png",
        "Metadata/thumbnail_small.png",
    ] {
        if let Some(bytes) = read_named(archive, names, candidate) {
            return Some(bytes);
        }
    }

    if let Some(rels) = read_text(archive, names, "_rels/.rels") {
        if let Some(target) = thumbnail_from_rels(&rels) {
            if let Some(bytes) = read_named(archive, names, &target) {
                return Some(bytes);
            }
        }
    }

    let mut fallback: Option<String> = None;
    for name in names {
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".png") {
            continue;
        }
        if lower.contains("pick_") || lower.contains("/top_") || lower.contains("top_plate") {
            continue;
        }
        if lower.contains("metadata/plate_") {
            return read_named(archive, names, name);
        }
        if fallback.is_none() {
            fallback = Some(name.clone());
        }
    }
    fallback.and_then(|name| read_named(archive, names, &name))
}

fn first_thumbnail_file(xml: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let key = "thumbnail_file";
    let idx = lower.find(key)?;
    let after = &xml[idx + key.len()..];
    let value_idx = after.to_ascii_lowercase().find("value=")?;
    let rest = after[value_idx + 6..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(
        inner[..end]
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string(),
    )
}

fn thumbnail_from_rels(rels: &str) -> Option<String> {
    let lower = rels.to_ascii_lowercase();
    let marker = "relationships/metadata/thumbnail";
    let idx = lower.find(marker)?;
    let window = &rels[..idx];
    let target_key = "target=";
    let t = window.to_ascii_lowercase().rfind(target_key)?;
    let rest = window[t + target_key.len()..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(
        inner[..end]
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string(),
    )
}

pub fn write_cover(file_id: i64, png: &[u8]) -> Result<String> {
    paths::ensure_app_dirs()?;
    let path = cover_abs(file_id)?;
    let mut file = fs::File::create(&path)?;
    file.write_all(png)?;
    Ok(cover_name(file_id))
}

pub fn apply_embedded_3mf(conn: &rusqlite::Connection, file_id: i64, abs_path: &Path) -> Result<Option<ThumbReady>> {
    let Some((png, plate_count)) = extract_3mf_cover(abs_path)? else {
        return Ok(None);
    };
    let name = write_cover(file_id, &png)?;
    db::update_thumb(conn, file_id, "embedded", Some(&name), Some(plate_count), None)?;
    Ok(Some(ThumbReady {
        id: file_id,
        thumb_path: Some(name),
        thumb_status: "embedded".into(),
        plate_count,
    }))
}

pub fn apply_companion_image(conn: &rusqlite::Connection, file_id: i64) -> Result<Option<ThumbReady>> {
    let Some(src) = db::image_companion_path(conn, file_id)? else {
        return Ok(None);
    };
    let bytes = fs::read(&src)?;
    if !bytes.starts_with(PNG_MAGIC)
        && !src.to_ascii_lowercase().ends_with(".jpg")
        && !src.to_ascii_lowercase().ends_with(".jpeg")
        && !src.to_ascii_lowercase().ends_with(".webp")
        && !src.to_ascii_lowercase().ends_with(".gif")
        && !src.to_ascii_lowercase().ends_with(".bmp")
    {
        return Ok(None);
    }
    let ext = Path::new(&src)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let name = if ext == "png" {
        write_cover(file_id, &bytes)?
    } else {
        let name = format!("{file_id}_cover.{ext}");
        paths::ensure_app_dirs()?;
        fs::write(thumbs_dir()?.join(&name), &bytes)?;
        name
    };
    db::update_thumb(conn, file_id, "embedded", Some(&name), None, None)?;
    Ok(Some(ThumbReady {
        id: file_id,
        thumb_path: Some(name),
        thumb_status: "embedded".into(),
        plate_count: 0,
    }))
}

pub fn save_rendered_png(conn: &rusqlite::Connection, file_id: i64, png: &[u8]) -> Result<ThumbReady> {
    if !png.starts_with(PNG_MAGIC) {
        return Err(AppError::Message("thumbnail is not a PNG".into()));
    }
    let name = write_cover(file_id, png)?;
    db::update_thumb(conn, file_id, "rendered", Some(&name), None, None)?;
    Ok(ThumbReady {
        id: file_id,
        thumb_path: Some(name),
        thumb_status: "rendered".into(),
        plate_count: 0,
    })
}

pub fn extract_pending(db: &Db) -> Result<Vec<ThumbReady>> {
    let pending = {
        let conn = db
            .0
            .lock()
            .map_err(|_| AppError::Message("database lock poisoned".into()))?;
        db::pending_thumbs(&conn)?
    };
    let mut ready = Vec::new();
    for file in pending {
        let conn = db
            .0
            .lock()
            .map_err(|_| AppError::Message("database lock poisoned".into()))?;
        let result = if file.ext == "3mf" {
            apply_embedded_3mf(&conn, file.id, Path::new(&file.abs_path))
        } else {
            apply_companion_image(&conn, file.id)
        };
        if let Ok(Some(thumb)) = result {
            ready.push(thumb);
        }
    }
    Ok(ready)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    fn write_3mf(path: &Path, settings: &str, png_name: &str) {
        let file = fs::File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default();
        zip.start_file("Metadata/model_settings.config", opts).unwrap();
        zip.write_all(settings.as_bytes()).unwrap();
        zip.start_file(png_name, opts).unwrap();
        zip.write_all(TINY_PNG).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn extracts_plate_png_and_counts_plates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("multi.3mf");
        write_3mf(
            &path,
            r#"<config>
              <plate>
                <metadata key="plater_id" value="1"/>
                <metadata key="thumbnail_file" value="Metadata/plate_1.png"/>
              </plate>
              <plate>
                <metadata key="plater_id" value="2"/>
              </plate>
            </config>"#,
            "Metadata/plate_1.png",
        );
        let (bytes, plates) = extract_3mf_cover(&path).unwrap().expect("cover");
        assert_eq!(bytes, TINY_PNG);
        assert_eq!(plates, 2);
    }

    #[test]
    fn parses_thumbnail_file_attr() {
        let xml = r#"<metadata key="thumbnail_file" value="Metadata/plate_4.png"/>"#;
        assert_eq!(
            first_thumbnail_file(xml).as_deref(),
            Some("Metadata/plate_4.png")
        );
    }
}
