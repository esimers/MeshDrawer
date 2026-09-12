use std::path::Path;

use crate::error::{AppError, Result};
use crate::threemf::MeshPart;

const DEFAULT_COLOR: [f32; 3] = [0.37, 0.92, 0.83];
const MAX_TRIANGLES: usize = 1_000_000;

pub fn tessellate_path(path: &Path, ext: &str) -> Result<Vec<MeshPart>> {
    match ext {
        "step" | "stp" => tessellate_step(path),
        "fbx" => tessellate_fbx(path),
        other => Err(AppError::Message(format!("no tessellator for .{other}"))),
    }
}

fn tessellate_step(path: &Path) -> Result<Vec<MeshPart>> {
    use truck_meshalgo::prelude::*;
    use truck_stepio::r#in::*;

    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => String::from_utf8_lossy(&std::fs::read(path)?).into_owned(),
    };
    let table = Table::from_step(&src)
        .ok_or_else(|| AppError::Message("could not parse STEP file".into()))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut any = false;
    for holder in table.shell.values() {
        let cshell = table
            .to_compressed_shell(holder)
            .map_err(|e| AppError::Message(format!("STEP shell convert: {e}")))?;
        let mut poly = cshell.robust_triangulation(0.05).to_polygon();
        poly.put_together_same_attrs(TOLERANCE * 50.0)
            .remove_degenerate_faces()
            .remove_unused_attrs();
        append_polygon(&poly, &mut vertices, &mut indices);
        any = true;
        if indices.len() / 3 >= MAX_TRIANGLES {
            break;
        }
    }
    if !any || vertices.is_empty() {
        return Err(AppError::Message(
            "STEP file has no tessellated faces".into(),
        ));
    }
    Ok(vec![MeshPart {
        object_id: 1,
        vertices,
        indices,
        color: DEFAULT_COLOR,
    }])
}

fn append_polygon(
    poly: &truck_polymesh::PolygonMesh,
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
) {
    let base = (vertices.len() / 3) as u32;
    for v in poly.positions() {
        vertices.extend_from_slice(&[v.x as f32, v.y as f32, v.z as f32]);
    }
    for face in poly.tri_faces() {
        if indices.len() / 3 >= MAX_TRIANGLES {
            break;
        }
        indices.extend_from_slice(&[
            base + face[0].pos as u32,
            base + face[1].pos as u32,
            base + face[2].pos as u32,
        ]);
    }
}

fn tessellate_fbx(path: &Path) -> Result<Vec<MeshPart>> {
    let filename = path
        .to_str()
        .ok_or_else(|| AppError::Message("FBX path is not valid UTF-8".into()))?;
    let scene = ufbx::load_file(filename, ufbx::LoadOpts::default()).map_err(|err| {
        AppError::Message(format!("FBX load failed: {}", err.description))
    })?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for node in &scene.nodes {
        if node.is_root {
            continue;
        }
        let Some(mesh) = node.mesh.as_ref() else {
            continue;
        };
        append_ufbx_mesh(mesh, &node.geometry_to_world, &mut vertices, &mut indices);
        if indices.len() / 3 >= MAX_TRIANGLES {
            break;
        }
    }
    if vertices.is_empty() {
        return Err(AppError::Message("FBX file has no mesh".into()));
    }
    Ok(vec![MeshPart {
        object_id: 1,
        vertices,
        indices,
        color: DEFAULT_COLOR,
    }])
}

fn append_ufbx_mesh(
    mesh: &ufbx::Mesh,
    transform: &ufbx::Matrix,
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
) {
    let mut tri = Vec::<u32>::new();
    for i in 0..mesh.faces.count {
        if indices.len() / 3 >= MAX_TRIANGLES {
            return;
        }
        let face = mesh.faces[i];
        let n = ufbx::triangulate_face_vec(&mut tri, mesh, face);
        for t in 0..(n as usize) {
            let idx = tri[t] as usize;
            let p = mesh.vertex_position[idx];
            let world = transform_point(transform, p.x, p.y, p.z);
            let out = (vertices.len() / 3) as u32;
            vertices.extend_from_slice(&world);
            indices.push(out);
        }
    }
}

fn transform_point(m: &ufbx::Matrix, x: f64, y: f64, z: f64) -> [f32; 3] {
    let px = m.m00 * x + m.m01 * y + m.m02 * z + m.m03;
    let py = m.m10 * x + m.m11 * y + m.m12 * z + m.m13;
    let pz = m.m20 * x + m.m21 * y + m.m22 * z + m.m23;
    [px as f32, py as f32, pz as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_ext() {
        let err = tessellate_path(Path::new("x.iges"), "iges").unwrap_err();
        assert!(err.to_string().contains("iges"));
    }
}
