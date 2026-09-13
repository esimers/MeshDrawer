use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::error::{AppError, Result};
use crate::threemf::MeshPart;

const DEFAULT_COLOR: [f32; 3] = [0.37, 0.92, 0.83];
const MAX_TRIANGLES: usize = 1_000_000;
const MAX_SHELLS: usize = 16;
const TESSELLATE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_STEP_BYTES: u64 = 40 * 1024 * 1024;
const PREVIEW_DIVISIONS: f64 = 100.0;
const PREVIEW_TOL_MIN: f64 = 0.5;
const PREVIEW_TOL_MAX: f64 = 20.0;

struct CacheEntry {
    key: String,
    parts: std::result::Result<Vec<MeshPart>, String>,
}

static LAST: Mutex<Option<CacheEntry>> = Mutex::new(None);
static CURRENT: Mutex<Option<String>> = Mutex::new(None);
static JOB: AtomicU64 = AtomicU64::new(0);

pub fn tessellate_path(path: &Path, ext: &str) -> Result<Vec<MeshPart>> {
    tessellate_path_with_job(path, ext, 0)
}

/// Tessellate off the caller thread and give up after [`TESSELLATE_TIMEOUT`].
/// Job `0` is not cancelled when the user switches files (thumbnail path).
pub fn tessellate_path_timed(path: &Path, ext: &str) -> Result<Vec<MeshPart>> {
    run_job(path, ext, 0)
}

/// Viewer preview: a newer request or [`cancel_request`] stops this job.
pub fn tessellate_path_for_request(
    path: &Path,
    ext: &str,
    request_id: &str,
) -> Result<Vec<MeshPart>> {
    if let Ok(mut current) = CURRENT.lock() {
        *current = Some(request_id.to_string());
    }
    let job = JOB.fetch_add(1, Ordering::SeqCst) + 1;
    run_job(path, ext, job)
}

pub fn cancel_request(request_id: &str) {
    if let Ok(mut current) = CURRENT.lock() {
        if current.as_deref() == Some(request_id) {
            *current = None;
            JOB.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn cancelled() -> AppError {
    AppError::Message("preview cancelled".into())
}

fn stale(job: u64) -> bool {
    job != 0 && JOB.load(Ordering::SeqCst) != job
}

fn run_job(path: &Path, ext: &str, job: u64) -> Result<Vec<MeshPart>> {
    let key = file_key(path);
    if let Some(parts) = cached(&key) {
        return parts;
    }
    if stale(job) {
        return Err(cancelled());
    }

    let path_buf = path.to_path_buf();
    let ext_owned = ext.to_string();
    let (tx, rx) = mpsc::sync_channel(1);
    if let Err(err) = thread::Builder::new()
        .name("tessellate".into())
        .spawn(move || {
            let caught = panic::catch_unwind(AssertUnwindSafe(|| {
                tessellate_path_with_job(&path_buf, &ext_owned, job)
            }));
            let mapped = match caught {
                Ok(Ok(parts)) => Ok(parts),
                Ok(Err(err)) => Err(err.to_string()),
                Err(_) => Err("CAD tessellator panicked on this file".into()),
            };
            if let Ok(parts) = &mapped {
                store_cache(file_key(&path_buf), Ok(parts.clone()));
            } else if let Err(msg) = &mapped {
                if !msg.contains("cancelled") {
                    store_cache(file_key(&path_buf), Err(msg.clone()));
                }
            }
            let _ = tx.send(mapped);
        })
    {
        return Err(AppError::Message(format!(
            "could not start tessellator: {err}"
        )));
    }

    let deadline = Instant::now() + TESSELLATE_TIMEOUT;
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(parts)) => {
                if stale(job) {
                    return Err(cancelled());
                }
                return Ok(parts);
            }
            Ok(Err(msg)) => {
                if msg.contains("cancelled") || stale(job) {
                    return Err(cancelled());
                }
                return Err(AppError::Message(msg));
            }
            Err(_) => {
                if stale(job) {
                    return Err(cancelled());
                }
                if Instant::now() >= deadline {
                    return Err(AppError::Message(format!(
                        "CAD tessellation timed out after {}s. The file is still indexed — open it in CAD or a slicer to inspect.",
                        TESSELLATE_TIMEOUT.as_secs()
                    )));
                }
            }
        }
    }
}

fn tessellate_path_with_job(path: &Path, ext: &str, job: u64) -> Result<Vec<MeshPart>> {
    if stale(job) {
        return Err(cancelled());
    }
    match ext {
        "step" | "stp" => tessellate_step(path, job),
        "fbx" => tessellate_fbx(path, job),
        other => Err(AppError::Message(format!("no tessellator for .{other}"))),
    }
}

fn file_key(path: &Path) -> String {
    let meta = std::fs::metadata(path).ok();
    let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}:{len}:{mtime}", path.display())
}

fn cached(key: &str) -> Option<Result<Vec<MeshPart>>> {
    let guard = LAST.lock().ok()?;
    let entry = guard.as_ref()?;
    if entry.key != key {
        return None;
    }
    Some(entry.parts.clone().map_err(AppError::Message))
}

fn store_cache(key: String, parts: std::result::Result<Vec<MeshPart>, String>) {
    if let Ok(mut guard) = LAST.lock() {
        *guard = Some(CacheEntry { key, parts });
    }
}

fn limit_tessellate_threads() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var_os("RAYON_NUM_THREADS").is_none() {
            std::env::set_var("RAYON_NUM_THREADS", "1");
        }
    });
}

fn preview_tol_xyz(points: impl IntoIterator<Item = (f64, f64, f64)>) -> f64 {
    let mut min = [f64::MAX; 3];
    let mut max = [f64::MIN; 3];
    let mut any = false;
    for (x, y, z) in points {
        any = true;
        min[0] = min[0].min(x);
        min[1] = min[1].min(y);
        min[2] = min[2].min(z);
        max[0] = max[0].max(x);
        max[1] = max[1].max(y);
        max[2] = max[2].max(z);
    }
    if !any {
        return 1.0;
    }
    let dx = max[0] - min[0];
    let dy = max[1] - min[1];
    let dz = max[2] - min[2];
    let diag = (dx * dx + dy * dy + dz * dz).sqrt().max(1.0);
    // Fine 0.05mm tessellation on a ~1m part tries to allocate tens of GB.
    (diag / PREVIEW_DIVISIONS).clamp(PREVIEW_TOL_MIN, PREVIEW_TOL_MAX)
}

fn tessellate_step(path: &Path, job: u64) -> Result<Vec<MeshPart>> {
    use truck_stepio::r#in::*;
    limit_tessellate_threads();

    let len = std::fs::metadata(path)?.len();
    if len > MAX_STEP_BYTES {
        return Err(AppError::Message(format!(
            "STEP file is too large to tessellate ({:.1} MB; cap is {} MB)",
            len as f64 / (1024.0 * 1024.0),
            MAX_STEP_BYTES / (1024 * 1024)
        )));
    }

    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => String::from_utf8_lossy(&std::fs::read(path)?).into_owned(),
    };
    let table = Table::from_step(&src)
        .ok_or_else(|| AppError::Message("could not parse STEP file".into()))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let deadline = Instant::now() + TESSELLATE_TIMEOUT;
    let mut shells = 0usize;
    for holder in table.shell.values() {
        if stale(job) {
            return Err(cancelled());
        }
        if Instant::now() >= deadline || shells >= MAX_SHELLS {
            break;
        }
        shells += 1;
        if let Some(poly) = tessellate_one_shell(&table, holder) {
            append_polygon(&poly, &mut vertices, &mut indices);
            if indices.len() / 3 >= MAX_TRIANGLES {
                break;
            }
        }
    }
    if vertices.is_empty() {
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

fn tessellate_one_shell(
    table: &truck_stepio::r#in::Table,
    holder: &truck_stepio::r#in::ShellHolder,
) -> Option<truck_polymesh::PolygonMesh> {
    use truck_meshalgo::prelude::*;

    panic::catch_unwind(AssertUnwindSafe(|| {
        let cshell = table.to_compressed_shell(holder).ok()?;
        let tol = preview_tol_xyz(cshell.vertices.iter().map(|v| (v.x, v.y, v.z)));
        let mut poly = cshell.triangulation(tol).to_polygon();
        if poly.positions().is_empty() {
            return None;
        }
        poly.put_together_same_attrs(TOLERANCE * 50.0)
            .remove_degenerate_faces()
            .remove_unused_attrs();
        Some(poly)
    }))
    .ok()
    .flatten()
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

fn tessellate_fbx(path: &Path, job: u64) -> Result<Vec<MeshPart>> {
    let filename = path
        .to_str()
        .ok_or_else(|| AppError::Message("FBX path is not valid UTF-8".into()))?;
    let scene = ufbx::load_file(filename, ufbx::LoadOpts::default()).map_err(|err| {
        AppError::Message(format!("FBX load failed: {}", err.description))
    })?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for node in &scene.nodes {
        if stale(job) {
            return Err(cancelled());
        }
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

    #[test]
    fn timed_rejects_unknown_ext() {
        let err = tessellate_path_timed(Path::new("x.iges"), "iges").unwrap_err();
        assert!(err.to_string().contains("iges"));
    }

    #[test]
    fn cancel_unmatched_request_is_a_noop() {
        cancel_request("missing");
        let err = tessellate_path_for_request(Path::new("x.iges"), "iges", "req-1").unwrap_err();
        assert!(err.to_string().contains("iges"));
    }

    #[test]
    fn preview_tol_stays_coarse_on_large_parts() {
        let small = preview_tol_xyz([(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)]);
        let large = preview_tol_xyz([(0.0, 0.0, 0.0), (2000.0, 0.0, 0.0)]);
        assert!(large > small);
        assert!(large <= PREVIEW_TOL_MAX);
        assert!(small >= PREVIEW_TOL_MIN);
        assert!(large >= 10.0);
    }
}
