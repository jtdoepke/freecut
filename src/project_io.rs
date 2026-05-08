//! Native Freecut project file persistence.
//!
//! Project files are explicit JSON documents. They are intentionally separate from the old
//! hidden `$HOME/.config/FreeCut` auto-save file.

use std::{fmt, fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    dim::MILLI_PER_UNIT,
    domain::PatternDirection,
    domain::{CutPiece, CutSettings, LayoutKind, LinearKerf, PieceId, Project, StockPiece, Unit},
    optimizer::OptimizerEffort,
};

pub const PROJECT_FILE_VERSION: u32 = 3;
pub const PROJECT_FILE_EXTENSION: &str = "freecut.json";

const SUPPORTED_PROJECT_FILE_VERSIONS: &[u32] = &[1, 2, 3];

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
    let value: serde_json::Value = serde_json::from_str(source)?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(1);

    if !SUPPORTED_PROJECT_FILE_VERSIONS.contains(&version) {
        return Err(ProjectIoError::UnsupportedVersion(version));
    }

    if version < PROJECT_FILE_VERSION {
        let legacy: LegacyProjectDocument = serde_json::from_value(value)?;
        return Ok(legacy.into_current());
    }

    let document: ProjectDocument = serde_json::from_value(value)?;
    Ok(document)
}

/// v1/v2 project document: dimension fields are in whole user units. Loaded then scaled to
/// milli-units for the current `ProjectDocument` shape.
#[derive(Debug, Deserialize)]
struct LegacyProjectDocument {
    #[serde(default)]
    version: u32,
    project: LegacyProject,
    optimizer_effort: OptimizerEffort,
}

#[derive(Debug, Deserialize)]
struct LegacyProject {
    name: String,
    stock_pieces: Vec<LegacyStockPiece>,
    cut_pieces: Vec<LegacyCutPiece>,
    settings: LegacyCutSettings,
}

#[derive(Debug, Deserialize)]
struct LegacyStockPiece {
    id: PieceId,
    width: u32,
    length: u32,
    quantity: Option<u32>,
    pattern: PatternDirection,
}

#[derive(Debug, Deserialize)]
struct LegacyCutPiece {
    id: PieceId,
    label: String,
    width: u32,
    length: u32,
    quantity: u32,
    pattern: PatternDirection,
    can_rotate: bool,
}

#[derive(Debug, Deserialize)]
struct LegacyCutSettings {
    unit: Unit,
    kerf_width: u32,
    #[serde(default)]
    linear_kerf: Option<LegacyLinearKerf>,
    layout: LayoutKind,
}

#[derive(Debug, Deserialize)]
struct LegacyLinearKerf {
    extra: u32,
    reference: u32,
}

impl LegacyProjectDocument {
    fn into_current(self) -> ProjectDocument {
        let _ = self.version; // kept for clarity; ignored after migration.
        let project = Project {
            name: self.project.name,
            stock_pieces: self
                .project
                .stock_pieces
                .into_iter()
                .map(|stock| StockPiece {
                    id: stock.id,
                    width: stock.width.saturating_mul(MILLI_PER_UNIT),
                    length: stock.length.saturating_mul(MILLI_PER_UNIT),
                    quantity: stock.quantity,
                    pattern: stock.pattern,
                })
                .collect(),
            cut_pieces: self
                .project
                .cut_pieces
                .into_iter()
                .map(|cut| CutPiece {
                    id: cut.id,
                    label: cut.label,
                    width: cut.width.saturating_mul(MILLI_PER_UNIT),
                    length: cut.length.saturating_mul(MILLI_PER_UNIT),
                    quantity: cut.quantity,
                    pattern: cut.pattern,
                    can_rotate: cut.can_rotate,
                })
                .collect(),
            settings: CutSettings {
                unit: self.project.settings.unit,
                kerf_width: self
                    .project
                    .settings
                    .kerf_width
                    .saturating_mul(MILLI_PER_UNIT),
                linear_kerf: self.project.settings.linear_kerf.map(|lk| LinearKerf {
                    extra: lk.extra.saturating_mul(MILLI_PER_UNIT),
                    reference: lk.reference.saturating_mul(MILLI_PER_UNIT),
                }),
                layout: self.project.settings.layout,
            },
        };

        ProjectDocument {
            version: PROJECT_FILE_VERSION,
            project,
            optimizer_effort: self.optimizer_effort,
        }
    }
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
                width: 2_440_000,
                length: 1_220_000,
                quantity: Some(2),
                pattern: PatternDirection::ParallelToLength,
            }],
            cut_pieces: Vec::new(),
            settings: CutSettings {
                unit: Unit::Millimeter,
                kerf_width: 3_000,
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
                    kerf_width: 1_000,
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

        assert_eq!(loaded.version, PROJECT_FILE_VERSION);
        assert_eq!(loaded.project.settings.kerf_width, 2_000);
        assert_eq!(loaded.project.settings.linear_kerf, None);
    }

    #[test]
    fn migrates_legacy_v2_project_dimensions_to_milli() {
        let source = r#"{
  "version": 2,
  "project": {
    "name": "v2 file",
    "stock_pieces": [
      {
        "id": 1,
        "width": 2440,
        "length": 1220,
        "quantity": 2,
        "pattern": "None"
      }
    ],
    "cut_pieces": [
      {
        "id": 2,
        "label": "side",
        "width": 700,
        "length": 500,
        "quantity": 4,
        "pattern": "ParallelToWidth",
        "can_rotate": false
      }
    ],
    "settings": {
      "unit": "Millimeter",
      "kerf_width": 3,
      "linear_kerf": { "extra": 1, "reference": 1000 },
      "layout": "Guillotine"
    }
  },
  "optimizer_effort": "Thorough"
}"#;

        let loaded = load_project_document_from_str(source).expect("v2 should migrate");

        assert_eq!(loaded.version, PROJECT_FILE_VERSION);
        assert_eq!(loaded.project.stock_pieces[0].width, 2_440_000);
        assert_eq!(loaded.project.stock_pieces[0].length, 1_220_000);
        assert_eq!(loaded.project.cut_pieces[0].width, 700_000);
        assert_eq!(loaded.project.cut_pieces[0].length, 500_000);
        assert_eq!(loaded.project.settings.kerf_width, 3_000);
        assert_eq!(
            loaded.project.settings.linear_kerf,
            Some(LinearKerf {
                extra: 1_000,
                reference: 1_000_000,
            })
        );
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
                    kerf_width: 1_000,
                    linear_kerf: Some(LinearKerf {
                        extra: 3_000,
                        reference: 1_000_000,
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
