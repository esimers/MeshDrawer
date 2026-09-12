use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

use crate::error::Result;

const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47];

#[derive(Debug, Clone)]
pub struct PlateExtract {
    pub index: i64,
    pub name: Option<String>,
    pub thumb_entry: Option<String>,
    pub thumb_png: Option<Vec<u8>>,
    pub object_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct MeshPart {
    pub object_id: u32,
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub color: [f32; 3],
}

pub fn normalize_zip_name(name: &str) -> String {
    name.replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

pub fn extract_plates(path: &Path) -> Result<Vec<PlateExtract>> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let names = zip_names(&mut archive);

    let settings = read_text(&mut archive, &names, "Metadata/model_settings.config");
    let mut plates = settings
        .as_deref()
        .map(parse_plate_blocks)
        .unwrap_or_default();

    if plates.is_empty() {
        plates = plates_from_pngs(&names);
    }
    if plates.is_empty() {
        plates.push(PlateExtract {
            index: 1,
            name: None,
            thumb_entry: None,
            thumb_png: None,
            object_ids: Vec::new(),
        });
    }

    for plate in &mut plates {
        if plate.thumb_png.is_none() {
            if let Some(entry) = &plate.thumb_entry {
                plate.thumb_png = read_png(&mut archive, &names, entry);
            }
        }
        if plate.thumb_png.is_none() {
            let guessed = format!("Metadata/plate_{}.png", plate.index);
            plate.thumb_png = read_png(&mut archive, &names, &guessed);
        }
    }

    Ok(plates)
}

pub fn load_plate_meshes(path: &Path, object_ids: &[u32]) -> Result<Vec<MeshPart>> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let names = zip_names(&mut archive);

    let mut objects: HashMap<u32, ParsedObject> = HashMap::new();
    if let Some(xml) = read_text(&mut archive, &names, "3D/3dmodel.model") {
        parse_model_objects(&xml, &mut objects);
    }

    let mut needed: std::collections::HashSet<u32> = if object_ids.is_empty() {
        names
            .iter()
            .filter_map(|n| object_id_from_part_path(n))
            .collect()
    } else {
        object_ids.iter().copied().collect()
    };
    if needed.is_empty() {
        needed.extend(objects.keys().copied());
    }
    let mut stack: Vec<u32> = needed.iter().copied().collect();
    while let Some(id) = stack.pop() {
        if let Some(ParsedObject::Components(children)) = objects.get(&id) {
            for child in children.clone() {
                if needed.insert(child) {
                    stack.push(child);
                }
            }
        }
        let object_path = format!("3D/Objects/object_{id}.model");
        if objects.contains_key(&id) {
            continue;
        }
        if let Some(xml) = read_text(&mut archive, &names, &object_path) {
            parse_model_objects(&xml, &mut objects);
            if let Some(ParsedObject::Components(children)) = objects.get(&id) {
                for child in children.clone() {
                    if needed.insert(child) {
                        stack.push(child);
                    }
                }
            }
        }
    }

    let colors = filament_colors(&mut archive, &names);
    let extruders = object_extruders(&mut archive, &names);

    let wanted: Vec<u32> = if object_ids.is_empty() {
        objects.keys().copied().collect()
    } else {
        object_ids.to_vec()
    };

    let mut parts = Vec::new();
    let mut tris = 0_u32;
    for id in wanted {
        for mut part in flatten_object(&objects, id, &mut Vec::new()) {
            let extruder = extruders.get(&part.object_id).copied().unwrap_or(1);
            part.color = color_for_extruder(&colors, extruder);
            let part_tris = if part.indices.is_empty() {
                (part.vertices.len() / 9) as u32
            } else {
                (part.indices.len() / 3) as u32
            };
            tris += part_tris;
            parts.push(part);
            if tris > 1_000_000 {
                break;
            }
        }
        if tris > 1_000_000 {
            break;
        }
    }
    Ok(parts)
}

pub fn encode_mesh_blob(parts: &[MeshPart]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"MD3M");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(parts.len() as u32).to_le_bytes());
    for part in parts {
        let vcount = (part.vertices.len() / 3) as u32;
        let icount = part.indices.len() as u32;
        out.extend_from_slice(&vcount.to_le_bytes());
        out.extend_from_slice(&icount.to_le_bytes());
        out.extend_from_slice(&part.color[0].to_le_bytes());
        out.extend_from_slice(&part.color[1].to_le_bytes());
        out.extend_from_slice(&part.color[2].to_le_bytes());
        for v in &part.vertices {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for i in &part.indices {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    out
}

fn object_id_from_part_path(name: &str) -> Option<u32> {
    let lower = normalize_zip_name(name).to_ascii_lowercase();
    let rest = lower.strip_prefix("3d/objects/object_")?;
    rest.strip_suffix(".model")?.parse().ok()
}

fn zip_names(archive: &mut ZipArchive<fs::File>) -> Vec<String> {
    (0..archive.len())
        .filter_map(|i| {
            archive
                .by_index(i)
                .ok()
                .map(|entry| normalize_zip_name(entry.name()))
        })
        .collect()
}

fn read_text(archive: &mut ZipArchive<fs::File>, names: &[String], want: &str) -> Option<String> {
    let idx = names.iter().position(|n| n.eq_ignore_ascii_case(want))?;
    let mut entry = archive.by_index(idx).ok()?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn read_png(archive: &mut ZipArchive<fs::File>, names: &[String], want: &str) -> Option<Vec<u8>> {
    let idx = names.iter().position(|n| n.eq_ignore_ascii_case(want))?;
    let mut entry = archive.by_index(idx).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    if buf.starts_with(PNG_MAGIC) {
        Some(buf)
    } else {
        None
    }
}

fn parse_plate_blocks(xml: &str) -> Vec<PlateExtract> {
    let mut plates = Vec::new();
    let lower = xml.to_ascii_lowercase();
    let mut search = 0;
    while let Some(start_rel) = lower[search..].find("<plate") {
        let start = search + start_rel;
        let Some(gt) = xml[start..].find('>') else {
            break;
        };
        if xml[start..start + gt].contains("/>") {
            search = start + gt + 1;
            continue;
        }
        let body_start = start + gt + 1;
        let Some(end_rel) = lower[body_start..].find("</plate>") else {
            break;
        };
        let block = &xml[body_start..body_start + end_rel];
        search = body_start + end_rel + 8;

        let index = meta_value(block, "plater_id")
            .and_then(|s| s.parse().ok())
            .unwrap_or((plates.len() as i64) + 1);
        let name = meta_value(block, "plater_name").filter(|s| !s.is_empty());
        let thumb_entry = meta_value(block, "thumbnail_file");
        let object_ids = object_ids_in_block(block);
        plates.push(PlateExtract {
            index,
            name,
            thumb_entry,
            thumb_png: None,
            object_ids,
        });
    }
    plates
}

fn plates_from_pngs(names: &[String]) -> Vec<PlateExtract> {
    let mut indices = Vec::new();
    for name in names {
        let lower = name.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("metadata/plate_") {
            if let Some(num) = rest
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|s| s.parse::<i64>().ok())
            {
                if !indices.contains(&num) {
                    indices.push(num);
                }
            }
        }
    }
    indices.sort_unstable();
    indices
        .into_iter()
        .map(|index| PlateExtract {
            index,
            name: None,
            thumb_entry: None,
            thumb_png: None,
            object_ids: Vec::new(),
        })
        .collect()
}

fn meta_value(block: &str, key: &str) -> Option<String> {
    let lower = block.to_ascii_lowercase();
    let needle = format!("key=\"{key}\"");
    let idx = lower.find(&needle)?;
    let after = &block[idx + needle.len()..];
    let v = after.to_ascii_lowercase().find("value=")?;
    let rest = after[v + 6..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

fn object_ids_in_block(block: &str) -> Vec<u32> {
    let lower = block.to_ascii_lowercase();
    let mut ids = Vec::new();
    let mut search = 0;
    while let Some(rel) = lower[search..].find("key=\"object_id\"") {
        let at = search + rel;
        if let Some(value) = meta_value(&block[at..], "object_id") {
            if let Ok(id) = value.parse::<u32>() {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        search = at + 10;
    }
    ids
}

#[derive(Debug, Clone)]
enum ParsedObject {
    Mesh { vertices: Vec<f32>, indices: Vec<u32> },
    Components(Vec<u32>),
}

fn parse_model_objects(xml: &str, into: &mut HashMap<u32, ParsedObject>) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current_id: Option<u32> = None;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut components = Vec::new();
    let mut in_mesh = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = e.local_name();
                let name = name.as_ref();
                if name == b"object" {
                    current_id = attr_u32(&e, b"id");
                    vertices.clear();
                    indices.clear();
                    components.clear();
                    in_mesh = false;
                } else if name == b"mesh" {
                    in_mesh = true;
                } else if name == b"vertex" && in_mesh {
                    if let (Some(x), Some(y), Some(z)) =
                        (attr_f32(&e, b"x"), attr_f32(&e, b"y"), attr_f32(&e, b"z"))
                    {
                        vertices.extend_from_slice(&[x, y, z]);
                    }
                } else if name == b"triangle" && in_mesh {
                    if let (Some(a), Some(b), Some(c)) =
                        (attr_u32(&e, b"v1"), attr_u32(&e, b"v2"), attr_u32(&e, b"v3"))
                    {
                        indices.extend_from_slice(&[a, b, c]);
                    }
                } else if name == b"component" {
                    if let Some(oid) = attr_u32(&e, b"objectid") {
                        components.push(oid);
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"object" {
                    if let Some(id) = current_id {
                        if !vertices.is_empty() {
                            into.insert(
                                id,
                                ParsedObject::Mesh {
                                    vertices: vertices.clone(),
                                    indices: indices.clone(),
                                },
                            );
                        } else if !components.is_empty() {
                            into.insert(id, ParsedObject::Components(components.clone()));
                        }
                    }
                    current_id = None;
                    in_mesh = false;
                } else if e.local_name().as_ref() == b"mesh" {
                    in_mesh = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

fn attr_f32(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<f32> {
    e.attributes()
        .with_checks(false)
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| a.unescape_value().ok())
        .and_then(|v| v.parse().ok())
}

fn attr_u32(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<u32> {
    e.attributes()
        .with_checks(false)
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| a.unescape_value().ok())
        .and_then(|v| v.parse().ok())
}

fn flatten_object(
    objects: &HashMap<u32, ParsedObject>,
    id: u32,
    stack: &mut Vec<u32>,
) -> Vec<MeshPart> {
    if stack.contains(&id) {
        return Vec::new();
    }
    stack.push(id);
    let mut out = Vec::new();
    match objects.get(&id) {
        Some(ParsedObject::Mesh { vertices, indices }) => out.push(MeshPart {
            object_id: id,
            vertices: vertices.clone(),
            indices: indices.clone(),
            color: [0.37, 0.92, 0.83],
        }),
        Some(ParsedObject::Components(children)) => {
            for child in children {
                out.extend(flatten_object(objects, *child, stack));
            }
        }
        None => {}
    }
    stack.pop();
    out
}

fn filament_colors(archive: &mut ZipArchive<fs::File>, names: &[String]) -> Vec<[f32; 3]> {
    let Some(text) = read_text(archive, names, "Metadata/project_settings.config") else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let raw = value.get("filament_colour");
    let mut colors = Vec::new();
    match raw {
        Some(serde_json::Value::Array(items)) => {
            for item in items {
                if let Some(s) = item.as_str() {
                    if let Some(c) = parse_hex_color(s) {
                        colors.push(c);
                    }
                }
            }
        }
        Some(serde_json::Value::String(s)) => {
            for part in s.split([';', ',']) {
                if let Some(c) = parse_hex_color(part.trim()) {
                    colors.push(c);
                }
            }
        }
        _ => {}
    }
    colors
}

fn object_extruders(archive: &mut ZipArchive<fs::File>, names: &[String]) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    let Some(xml) = read_text(archive, names, "Metadata/model_settings.config") else {
        return map;
    };
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current: Option<u32> = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.local_name().as_ref() == b"object" {
                    current = attr_u32(&e, b"id");
                } else if e.local_name().as_ref() == b"metadata" {
                    if let (Some(id), Some(key), Some(val)) =
                        (current, attr_string(&e, b"key"), attr_string(&e, b"value"))
                    {
                        if key.eq_ignore_ascii_case("extruder") {
                            if let Ok(n) = val.parse::<u32>() {
                                map.insert(id, n.max(1));
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"object" {
                    current = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

fn attr_string(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    e.attributes()
        .with_checks(false)
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| a.unescape_value().ok())
        .map(|v| v.into_owned())
}

fn parse_hex_color(s: &str) -> Option<[f32; 3]> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

fn color_for_extruder(colors: &[[f32; 3]], extruder: u32) -> [f32; 3] {
    colors
        .get(extruder.saturating_sub(1) as usize)
        .copied()
        .unwrap_or([0.37, 0.92, 0.83])
}

#[cfg(test)]
fn decode_mesh_blob_for_test(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 12 || &bytes[0..4] != b"MD3M" {
        return None;
    }
    let nparts = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
    Some(nparts as usize)
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

    #[test]
    fn parses_two_plates_and_object_ids() {
        let xml = r#"<config>
          <plate>
            <metadata key="plater_id" value="1"/>
            <metadata key="plater_name" value="Body"/>
            <model_instance>
              <metadata key="object_id" value="2"/>
            </model_instance>
          </plate>
          <plate>
            <metadata key="plater_id" value="2"/>
            <model_instance>
              <metadata key="object_id" value="5"/>
            </model_instance>
          </plate>
        </config>"#;
        let plates = parse_plate_blocks(xml);
        assert_eq!(plates.len(), 2);
        assert_eq!(plates[0].name.as_deref(), Some("Body"));
        assert_eq!(plates[0].object_ids, vec![2]);
        assert_eq!(plates[1].object_ids, vec![5]);
    }

    #[test]
    fn extracts_plate_pngs_from_zip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("p.3mf");
        let file = fs::File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default();
        zip.start_file("Metadata/model_settings.config", opts).unwrap();
        zip.write_all(
            br#"<config>
              <plate><metadata key="plater_id" value="1"/><metadata key="thumbnail_file" value="Metadata/plate_1.png"/></plate>
              <plate><metadata key="plater_id" value="2"/><metadata key="thumbnail_file" value="Metadata/plate_2.png"/></plate>
            </config>"#,
        )
        .unwrap();
        zip.start_file("Metadata/plate_1.png", opts).unwrap();
        zip.write_all(TINY_PNG).unwrap();
        zip.start_file("Metadata/plate_2.png", opts).unwrap();
        zip.write_all(TINY_PNG).unwrap();
        zip.finish().unwrap();

        let plates = extract_plates(&path).unwrap();
        assert_eq!(plates.len(), 2);
        assert!(plates[0].thumb_png.is_some());
        assert!(plates[1].thumb_png.is_some());
    }

    #[test]
    fn parses_mesh_and_encodes_blob() {
        let xml = r#"<?xml version="1.0"?>
        <model>
          <resources>
            <object id="7" type="model">
              <mesh>
                <vertices>
                  <vertex x="0" y="0" z="0"/>
                  <vertex x="1" y="0" z="0"/>
                  <vertex x="0" y="1" z="0"/>
                </vertices>
                <triangles>
                  <triangle v1="0" v2="1" v3="2"/>
                </triangles>
              </mesh>
            </object>
          </resources>
        </model>"#;
        let mut objects = HashMap::new();
        parse_model_objects(xml, &mut objects);
        let parts = flatten_object(&objects, 7, &mut Vec::new());
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].vertices.len(), 9);
        assert_eq!(parts[0].indices, vec![0, 1, 2]);
        let blob = encode_mesh_blob(&parts);
        assert_eq!(decode_mesh_blob_for_test(&blob), Some(1));
    }
}
