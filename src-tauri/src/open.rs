use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlicerApp {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub available: bool,
}

fn existing(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(PathBuf::from).find(|p| p.is_file())
}

pub fn detected_slicers() -> Vec<SlicerApp> {
    detected_slicers_with(None, None)
}

pub fn detected_slicers_with(bambu_override: Option<&str>, orca_override: Option<&str>) -> Vec<SlicerApp> {
    let bambu = bambu_override
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| existing(&[
        r"C:\Program Files\Bambu Studio\bambu-studio.exe",
        r"C:\Program Files\BambuLab\Bambu Studio\bambu-studio.exe",
        r"C:\Program Files\Bambu Lab\Bambu Studio\bambu-studio.exe",
        r"C:\Program Files (x86)\Bambu Studio\bambu-studio.exe",
        ]));
    let orca = orca_override
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| existing(&[
        r"C:\Program Files\OrcaSlicer\orca-slicer.exe",
        r"C:\Program Files\Orca Slicer\orca-slicer.exe",
        r"C:\Program Files (x86)\OrcaSlicer\orca-slicer.exe",
        ]));
    vec![
        SlicerApp {
            id: "bambu".into(),
            name: "Bambu Studio".into(),
            available: bambu.is_some(),
            path: bambu.map(|p| p.to_string_lossy().into_owned()),
        },
        SlicerApp {
            id: "orca".into(),
            name: "Orca Slicer".into(),
            available: orca.is_some(),
            path: orca.map(|p| p.to_string_lossy().into_owned()),
        },
        SlicerApp {
            id: "system".into(),
            name: "System default".into(),
            available: true,
            path: None,
        },
    ]
}

pub fn open_with(slicer_id: &str, model: &Path) -> Result<()> {
    if !model.exists() {
        return Err(AppError::path("file does not exist", model));
    }
    match slicer_id {
        "bambu" | "orca" => {
            let slicer = detected_slicers()
                .into_iter()
                .find(|s| s.id == slicer_id && s.available)
                .and_then(|s| s.path)
                .ok_or_else(|| AppError::Message(format!("{slicer_id} was not found")))?;
            Command::new(slicer)
                .arg(model)
                .spawn()
                .map_err(|e| AppError::Message(e.to_string()))?;
        }
        _ => {
            Command::new("cmd")
                .args(["/c", "start", "", &model.to_string_lossy()])
                .spawn()
                .map_err(|e| AppError::Message(e.to_string()))?;
        }
    }
    Ok(())
}

pub fn reveal(model: &Path) -> Result<()> {
    if !model.exists() {
        return Err(AppError::path("file does not exist", model));
    }
    Command::new("explorer")
        .arg(format!("/select,{}", model.display()))
        .spawn()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn windows_properties(model: &Path) -> Result<()> {
    if !model.exists() {
        return Err(AppError::path("file does not exist", model));
    }
    let path = model.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$p = '{path}'; $s = New-Object -ComObject Shell.Application; $f = $s.NameSpace([IO.Path]::GetDirectoryName($p)); $i = $f.ParseName([IO.Path]::GetFileName($p)); if ($i) {{ $i.InvokeVerb('Properties') }}; Start-Sleep -Seconds 60"
    );
    Command::new("powershell")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_slicer_is_always_listed() {
        let slicers = detected_slicers();
        assert!(slicers.iter().any(|s| s.id == "system" && s.available));
        assert!(slicers.iter().any(|s| s.id == "bambu"));
        assert!(slicers.iter().any(|s| s.id == "orca"));
    }
}
