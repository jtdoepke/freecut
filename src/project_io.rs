//! Native Freecut project file persistence.
//!
//! Project files are explicit JSON documents. They are intentionally separate from the old
//! hidden `$HOME/.config/FreeCut` auto-save file.

use std::{fmt, fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::{domain::Project, optimizer::OptimizerEffort};

pub const PROJECT_FILE_VERSION: u32 = 2;
pub const PROJECT_FILE_EXTENSION: &str = "freecut.json";

const SUPPORTED_PROJECT_FILE_VERSIONS: &[u32] = &[1, 2];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub version: u32,
    pub project: Project,
    pub optimizer_effort: OptimizerEffort,
}

impl ProjectDocument {
    #[must_use]
    pub fn new(project: Project, optimizer_effort: OptimizerEffort) -> Self {
        Self {
            version: PROJECT_FILE_VERSION,
            project,
            optimizer_effort,
        }
    }
}

#[derive(Debug)]
pub enum ProjectIoError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
}

impl fmt::Display for ProjectIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "Dateifehler: {error}"),
            Self::Json(error) => write!(formatter, "Projektdatei ist kein gültiges JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "Projektdatei-Version {version} wird nicht unterstützt"
                )
            }
        }
    }
}

impl std::error::Error for ProjectIoError {}

impl From<io::Error> for ProjectIoError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ProjectIoError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn save_project_file(
    path: impl AsRef<Path>,
    document: &ProjectDocument,
) -> Result<(), ProjectIoError> {
    let mut serialized = serde_json::to_string_pretty(document)?;
    serialized.push('\n');
    fs::write(path, serialized)?;
    Ok(())
}

#[allow(clippy::missing_errors_doc)]
pub fn load_project_file(path: impl AsRef<Path>) -> Result<ProjectDocument, ProjectIoError> {
    let source = fs::read_to_string(path)?;
    load_project_document_from_str(&source)
}

#[allow(clippy::missing_errors_doc)]
pub fn load_project_document_from_str(source: &str) -> Result<ProjectDocument, ProjectIoError> {
    let document = serde_json::from_str::<ProjectDocument>(source)?;

    if !SUPPORTED_PROJECT_FILE_VERSIONS.contains(&document.version) {
        return Err(ProjectIoError::UnsupportedVersion(document.version));
    }

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        CutSettings, LayoutKind, LinearKerf, PatternDirection, PieceId, StockPiece, Unit,
    };

    #[test]
    fn project_document_roundtrip_preserves_project_and_effort() {
        let project = Project {
            name: "roundtrip".to_string(),
            stock_pieces: vec![StockPiece {
                id: PieceId(1),
                width: 2440,
                length: 1220,
                quantity: Some(2),
                pattern: PatternDirection::ParallelToLength,
            }],
            cut_pieces: Vec::new(),
            settings: CutSettings {
                unit: Unit::Millimeter,
                kerf_width: 3,
                linear_kerf: None,
                layout: LayoutKind::Guillotine,
            },
        };
        let document = ProjectDocument::new(project, OptimizerEffort::Thorough);
        let serialized = serde_json::to_string_pretty(&document).expect("serialize project");

        let loaded = load_project_document_from_str(&serialized).expect("load project");

        assert_eq!(loaded, document);
    }

    #[test]
    fn rejects_unsupported_project_file_version() {
        let source = r#"{
  "version": 999,
  "project": {
    "name": "too new",
    "stock_pieces": [],
    "cut_pieces": [],
    "settings": { "unit": "Millimeter", "kerf_width": 0, "layout": "Guillotine" }
  },
  "optimizer_effort": "Fast"
}"#;

        let error = load_project_document_from_str(source).expect_err("version should fail");

        assert_eq!(
            error.to_string(),
            "Projektdatei-Version 999 wird nicht unterstützt"
        );
    }

    #[test]
    fn save_and_load_project_file_roundtrip() {
        let path = std::env::temp_dir().join(format!(
            "freecut-project-{}-{}.{}",
            std::process::id(),
            PROJECT_FILE_VERSION,
            PROJECT_FILE_EXTENSION
        ));
        let document = ProjectDocument::new(
            Project {
                name: "file roundtrip".to_string(),
                stock_pieces: Vec::new(),
                cut_pieces: Vec::new(),
                settings: CutSettings {
                    unit: Unit::Foot,
                    kerf_width: 1,
                    linear_kerf: None,
                    layout: LayoutKind::Guillotine,
                },
            },
            OptimizerEffort::Balanced,
        );

        save_project_file(&path, &document).expect("save project");
        let loaded = load_project_file(&path).expect("load project");
        std::fs::remove_file(path).expect("remove project fixture");

        assert_eq!(loaded, document);
    }

    #[test]
    fn loads_legacy_v1_project_without_linear_kerf_field() {
        let source = r#"{
  "version": 1,
  "project": {
    "name": "legacy",
    "stock_pieces": [],
    "cut_pieces": [],
    "settings": { "unit": "Millimeter", "kerf_width": 2, "layout": "Guillotine" }
  },
  "optimizer_effort": "Balanced"
}"#;

        let loaded = load_project_document_from_str(source).expect("v1 project should load");

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.project.settings.kerf_width, 2);
        assert_eq!(loaded.project.settings.linear_kerf, None);
    }

    #[test]
    fn linear_kerf_roundtrip_preserves_extra_and_reference() {
        let document = ProjectDocument::new(
            Project {
                name: "linear-kerf".to_string(),
                stock_pieces: Vec::new(),
                cut_pieces: Vec::new(),
                settings: CutSettings {
                    unit: Unit::Millimeter,
                    kerf_width: 1,
                    linear_kerf: Some(LinearKerf {
                        extra: 3,
                        reference: 1000,
                    }),
                    layout: LayoutKind::Guillotine,
                },
            },
            OptimizerEffort::Balanced,
        );

        let serialized = serde_json::to_string_pretty(&document).expect("serialize");
        let loaded = load_project_document_from_str(&serialized).expect("load");

        assert_eq!(loaded, document);
        assert_eq!(loaded.version, PROJECT_FILE_VERSION);
    }
}
