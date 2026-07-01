#![allow(dead_code)]

use crate::adapters::{adapter_for, tool_rules::tool_rule, AdapterError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum DrillError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Hex(String),
    DetectorNotFound(String),
    ExternalAdapter(String),
}

impl From<std::io::Error> for DrillError {
    fn from(err: std::io::Error) -> Self {
        DrillError::Io(err)
    }
}

impl From<serde_json::Error> for DrillError {
    fn from(err: serde_json::Error) -> Self {
        DrillError::Json(err)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatDatabase {
    pub version: u32,
    pub signatures: Vec<CheatSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSignature {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub subtype: String,
    pub magic: String,
    pub offset: OffsetRule,
    pub detector: String,
    pub dependency: String,
    #[serde(default)]
    pub external_tool: Option<String>,
    pub base_confidence: u8,
    pub validation_required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OffsetRule {
    Any(String),
    Fixed(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_format: String,
    pub original_file: String,
    pub total_size: usize,
    pub sha256: String,
    pub root: Node,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: String,
    pub subtype: String,
    pub start_offset: usize,
    pub size: usize,
    pub raw_hash: String,
    pub detector_used: String,
    pub dependency: String,
    pub confidence: Confidence,
    pub metadata: NodeMetadata,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confidence {
    pub level: String,
    pub score: u8,
    pub criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub validated: bool,
    pub reversible: bool,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub signature: CheatSignature,
    pub offset: usize,
    pub magic_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub valid: bool,
    pub detected_size: usize,
    pub final_score: u8,
    pub criteria: Vec<String>,
    pub error: Option<String>,
    pub extracted_children: Vec<ExtractedChild>,
}

#[derive(Debug, Clone)]
pub struct ExtractedChild {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AnalysisLimits {
    pub max_depth: usize,
    pub max_children_per_node: usize,
    pub max_extracted_size: usize,
}

impl Default for AnalysisLimits {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_children_per_node: 10_000,
            max_extracted_size: 256 * 1024 * 1024,
        }
    }
}

pub struct DrillEngine {
    database: CheatDatabase,
    limits: AnalysisLimits,
}

impl DrillEngine {
    pub fn from_builtin_database() -> Result<Self, DrillError> {
        Self::from_database_json(include_str!("../cheat_db.json"))
    }

    pub fn from_database_file(path: impl AsRef<Path>) -> Result<Self, DrillError> {
        let json = fs::read_to_string(path)?;
        Self::from_database_json(&json)
    }

    pub fn from_database_source(source: &str, local_path: &str) -> Result<Self, DrillError> {
        if source == "local" && !local_path.is_empty() {
            Self::from_database_file(local_path)
        } else {
            Self::from_builtin_database()
        }
    }

    pub fn from_database_json(json: &str) -> Result<Self, DrillError> {
        let database: CheatDatabase = serde_json::from_str(&json)?;

        Ok(Self {
            database,
            limits: AnalysisLimits::default(),
        })
    }

    pub fn with_limits(mut self, limits: AnalysisLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn analyze_file(&self, file: impl AsRef<Path>) -> Result<Manifest, DrillError> {
        let file = file.as_ref();
        let bytes = fs::read(file)?;
        let hash = sha256_hex(&bytes);

        let mut root = Node {
            id: "n0000".to_string(),
            node_type: "root".to_string(),
            subtype: "original_file".to_string(),
            start_offset: 0,
            size: bytes.len(),
            raw_hash: hash.clone(),
            detector_used: "drill_root".to_string(),
            dependency: "none".to_string(),
            confidence: Confidence {
                level: "high".to_string(),
                score: 100,
                criteria: vec!["file_opened".to_string(), "sha256_calculated".to_string()],
            },
            metadata: NodeMetadata {
                name: file
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string()),
                description: Some("Original file analyzed by Drill".to_string()),
                validated: true,
                reversible: false,
                validation_error: None,
            },
            children: Vec::new(),
        };

        let mut id_counter = 1;
        self.analyze_bytes_recursive(&mut root, &bytes, 0, &mut id_counter)?;

        Ok(Manifest {
            manifest_format: "drill_manifest_v1".to_string(),
            original_file: file.to_string_lossy().to_string(),
            total_size: bytes.len(),
            sha256: hash,
            root,
        })
    }

    fn analyze_bytes_recursive(
        &self,
        node: &mut Node,
        bytes: &[u8],
        depth: usize,
        id_counter: &mut usize,
    ) -> Result<(), DrillError> {
        if depth >= self.limits.max_depth {
            return Ok(());
        }

        let candidates = self.find_candidates(bytes)?;

        for candidate in candidates {
            if node.children.len() >= self.limits.max_children_per_node {
                break;
            }

            let result = self.validate_candidate(bytes, &candidate)?;
            let level = confidence_level(result.valid, result.final_score);

            if !result.valid && candidate.signature.validation_required {
                continue;
            }

            let size = result
                .detected_size
                .min(bytes.len().saturating_sub(candidate.offset));

            if size == 0 {
                continue;
            }

            let end = candidate.offset.saturating_add(size).min(bytes.len());
            let slice = &bytes[candidate.offset..end];

            let mut child = Node {
                id: format!("n{:04}", *id_counter),
                node_type: candidate.signature.node_type.clone(),
                subtype: candidate.signature.subtype.clone(),
                start_offset: candidate.offset,
                size,
                raw_hash: sha256_hex(slice),
                detector_used: candidate.signature.detector.clone(),
                dependency: candidate.signature.dependency.clone(),
                confidence: Confidence {
                    level: level.to_string(),
                    score: result.final_score,
                    criteria: result.criteria.clone(),
                },
                metadata: NodeMetadata {
                    name: Some(candidate.signature.name.clone()),
                    description: Some(candidate.signature.description.clone()),
                    validated: result.valid,
                    reversible: false,
                    validation_error: result.error.clone(),
                },
                children: Vec::new(),
            };

            *id_counter += 1;

            for extracted in result.extracted_children {
                if extracted.bytes.len() > self.limits.max_extracted_size {
                    continue;
                }

                let mut extracted_child = Node {
                    id: format!("n{:04}", *id_counter),
                    node_type: "extracted_file".to_string(),
                    subtype: "unknown".to_string(),
                    start_offset: 0,
                    size: extracted.bytes.len(),
                    raw_hash: sha256_hex(&extracted.bytes),
                    detector_used: "extracted_by_dependency".to_string(),
                    dependency: candidate.signature.dependency.clone(),
                    confidence: Confidence {
                        level: "medium".to_string(),
                        score: 70,
                        criteria: vec![
                            "extracted_from_validated_container".to_string(),
                            "pending_recursive_analysis".to_string(),
                        ],
                    },
                    metadata: NodeMetadata {
                        name: Some(extracted.name),
                        description: Some("File extracted from container".to_string()),
                        validated: true,
                        reversible: false,
                        validation_error: None,
                    },
                    children: Vec::new(),
                };

                *id_counter += 1;

                self.analyze_bytes_recursive(
                    &mut extracted_child,
                    &extracted.bytes,
                    depth + 1,
                    id_counter,
                )?;

                child.children.push(extracted_child);
            }

            node.children.push(child);
        }

        Ok(())
    }

    fn find_candidates(&self, bytes: &[u8]) -> Result<Vec<Candidate>, DrillError> {
        let mut candidates = Vec::new();

        for signature in &self.database.signatures {
            let magic_bytes = parse_hex_magic(&signature.magic)?;

            match &signature.offset {
                OffsetRule::Fixed(offset) => {
                    if bytes_match_at(bytes, &magic_bytes, *offset) {
                        candidates.push(Candidate {
                            signature: signature.clone(),
                            offset: *offset,
                            magic_bytes,
                        });
                    }
                }
                OffsetRule::Any(value) => {
                    if value == "any" {
                        for offset in find_all(bytes, &magic_bytes) {
                            candidates.push(Candidate {
                                signature: signature.clone(),
                                offset,
                                magic_bytes: magic_bytes.clone(),
                            });
                        }
                    }
                }
            }
        }

        candidates.sort_by_key(|candidate| candidate.offset);
        Ok(candidates)
    }

    fn validate_candidate(
        &self,
        bytes: &[u8],
        candidate: &Candidate,
    ) -> Result<DetectionResult, DrillError> {
        match candidate.signature.detector.as_str() {
            "zip_detector" => Ok(validate_zip(bytes, candidate)),
            "png_detector" => Ok(validate_png(bytes, candidate)),
            "elf_detector" => Ok(validate_elf(bytes, candidate)),
            "java_class_detector" => Ok(validate_java_class(bytes, candidate)),
            "generic_magic_detector" => Ok(validate_generic_magic(bytes, candidate)),
            "bmp_detector" => Ok(validate_bmp(bytes, candidate)),
            "external_tool_detector"
            | "uefi_extract_detector"
            | "binwalk_detector"
            | "ghidra_detector" => Ok(validate_external_tool(bytes, candidate)),
            other => Err(DrillError::DetectorNotFound(other.to_string())),
        }
    }

    pub fn save_manifest(
        &self,
        manifest: &Manifest,
        destination: impl AsRef<Path>,
    ) -> Result<(), DrillError> {
        let json = serde_json::to_string_pretty(manifest)?;
        fs::write(destination, json)?;
        Ok(())
    }
}

fn validate_zip(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    let slice = &bytes[candidate.offset..];
    let cursor = Cursor::new(slice);

    match zip::ZipArchive::new(cursor) {
        Ok(mut archive) => {
            let mut children = Vec::new();

            for index in 0..archive.len() {
                if let Ok(mut file) = archive.by_index(index) {
                    if file.is_dir() {
                        continue;
                    }

                    let mut content = Vec::new();

                    if file.read_to_end(&mut content).is_ok() {
                        children.push(ExtractedChild {
                            name: file.name().to_string(),
                            bytes: content,
                        });
                    }
                }
            }

            DetectionResult {
                valid: true,
                detected_size: slice.len(),
                final_score: 95,
                criteria: vec![
                    "signature_found".to_string(),
                    "zip_validated".to_string(),
                    "entries_extracted".to_string(),
                ],
                error: None,
                extracted_children: children,
            }
        }
        Err(err) => DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: candidate.signature.base_confidence,
            criteria: vec!["signature_found".to_string()],
            error: Some(format!("ZIP validation failed: {err}")),
            extracted_children: Vec::new(),
        },
    }
}

fn validate_png(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    let valid = bytes_match_at(bytes, &candidate.magic_bytes, candidate.offset);

    DetectionResult {
        valid,
        detected_size: estimate_size_to_end(bytes, candidate.offset),
        final_score: if valid { 90 } else { 30 },
        criteria: if valid {
            vec![
                "png_signature_found".to_string(),
                "magic_bytes_valid".to_string(),
            ]
        } else {
            vec!["invalid_signature".to_string()]
        },
        error: None,
        extracted_children: Vec::new(),
    }
}

fn validate_bmp(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    if !bytes_match_at(bytes, &candidate.magic_bytes, candidate.offset) {
        return DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: 20,
            criteria: vec!["invalid_signature".to_string()],
            error: Some("Invalid BMP magic".to_string()),
            extracted_children: Vec::new(),
        };
    }

    if bytes.len() >= candidate.offset + 6 {
        let size_bytes = &bytes[candidate.offset + 2..candidate.offset + 6];
        let size = u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]])
            as usize;

        let valid = size > 0 && candidate.offset + size <= bytes.len();

        return DetectionResult {
            valid,
            detected_size: if valid {
                size
            } else {
                candidate.magic_bytes.len()
            },
            final_score: if valid { 90 } else { 55 },
            criteria: if valid {
                vec![
                    "bmp_signature_found".to_string(),
                    "bmp_size_validated".to_string(),
                ]
            } else {
                vec!["bmp_signature_found".to_string()]
            },
            error: if valid {
                None
            } else {
                Some("BMP found, but declared size is invalid".to_string())
            },
            extracted_children: Vec::new(),
        };
    }

    DetectionResult {
        valid: false,
        detected_size: candidate.magic_bytes.len(),
        final_score: 40,
        criteria: vec!["bmp_signature_found".to_string()],
        error: Some("File is too short to validate BMP".to_string()),
        extracted_children: Vec::new(),
    }
}

fn validate_elf(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    if !bytes_match_at(bytes, &candidate.magic_bytes, candidate.offset) {
        return DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: 20,
            criteria: vec!["invalid_signature".to_string()],
            error: Some("Invalid ELF magic".to_string()),
            extracted_children: Vec::new(),
        };
    }

    let minimum_header = candidate.offset + 16;
    let valid = bytes.len() >= minimum_header;

    DetectionResult {
        valid,
        detected_size: estimate_size_to_end(bytes, candidate.offset),
        final_score: if valid { 90 } else { 50 },
        criteria: if valid {
            vec![
                "elf_signature_found".to_string(),
                "minimum_header_present".to_string(),
            ]
        } else {
            vec!["elf_signature_found".to_string()]
        },
        error: if valid {
            None
        } else {
            Some("Incomplete ELF header".to_string())
        },
        extracted_children: Vec::new(),
    }
}

fn validate_java_class(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    if !bytes_match_at(bytes, &candidate.magic_bytes, candidate.offset) {
        return DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: 20,
            criteria: vec!["invalid_signature".to_string()],
            error: Some("Invalid Java class magic".to_string()),
            extracted_children: Vec::new(),
        };
    }

    let valid = bytes.len() >= candidate.offset + 10;

    DetectionResult {
        valid,
        detected_size: estimate_size_to_end(bytes, candidate.offset),
        final_score: if valid { 90 } else { 50 },
        criteria: if valid {
            vec![
                "cafebabe_signature_found".to_string(),
                "minimum_class_header_present".to_string(),
            ]
        } else {
            vec!["cafebabe_signature_found".to_string()]
        },
        error: if valid {
            None
        } else {
            Some("Incomplete Java .class header".to_string())
        },
        extracted_children: Vec::new(),
    }
}

fn validate_generic_magic(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    let valid = bytes_match_at(bytes, &candidate.magic_bytes, candidate.offset);

    DetectionResult {
        valid,
        detected_size: estimate_size_to_end(bytes, candidate.offset),
        final_score: if valid {
            candidate.signature.base_confidence
        } else {
            10
        },
        criteria: if valid {
            vec!["signature_found".to_string()]
        } else {
            vec!["invalid_signature".to_string()]
        },
        error: None,
        extracted_children: Vec::new(),
    }
}

fn validate_external_tool(bytes: &[u8], candidate: &Candidate) -> DetectionResult {
    let Some(tool) = candidate.signature.external_tool.as_deref() else {
        return DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: 10,
            criteria: vec!["external_tool_missing".to_string()],
            error: Some("Signature requires an external tool, but none was configured".to_string()),
            extracted_children: Vec::new(),
        };
    };

    let Some(adapter) = adapter_for(tool) else {
        return DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: 10,
            criteria: vec!["external_adapter_missing".to_string()],
            error: Some(format!("No external adapter registered for {tool}")),
            extracted_children: Vec::new(),
        };
    };

    let workspace = external_workspace(adapter.name(), candidate.offset);
    let input_path = workspace.join("input.bin");

    let result = (|| -> Result<_, DrillError> {
        fs::create_dir_all(&workspace)?;
        fs::write(&input_path, bytes)?;
        adapter
            .analyze(&input_path, &workspace)
            .map_err(|err| DrillError::ExternalAdapter(format_adapter_error(&err)))
    })();

    let _ = fs::remove_dir_all(&workspace);

    match result {
        Ok(analysis) => {
            let extracted_children = analysis
                .artifacts
                .into_iter()
                .map(|artifact| ExtractedChild {
                    name: artifact.name,
                    bytes: artifact.bytes,
                })
                .collect::<Vec<_>>();

            DetectionResult {
                valid: true,
                detected_size: estimate_size_to_end(bytes, candidate.offset),
                final_score: 90,
                criteria: vec![
                    "signature_found".to_string(),
                    format!("external_adapter:{}", adapter.name()),
                    format!("external_command:{}", analysis.command),
                    format!("external_status:{:?}", analysis.status_code),
                    format!("artifacts_extracted:{}", extracted_children.len()),
                ],
                error: None,
                extracted_children,
            }
        }
        Err(err) => DetectionResult {
            valid: false,
            detected_size: candidate.magic_bytes.len(),
            final_score: candidate.signature.base_confidence,
            criteria: vec![
                "signature_found".to_string(),
                format!("external_adapter:{}", adapter.name()),
            ],
            error: Some(format!("{err:?}")),
            extracted_children: Vec::new(),
        },
    }
}

fn parse_hex_magic(input: &str) -> Result<Vec<u8>, DrillError> {
    let clean = input.split_whitespace().collect::<Vec<_>>().join("");

    if clean.len() % 2 != 0 {
        return Err(DrillError::Hex(format!("Invalid magic value: {input}")));
    }

    hex::decode(clean).map_err(|_| DrillError::Hex(format!("Invalid magic value: {input}")))
}

fn bytes_match_at(bytes: &[u8], magic: &[u8], offset: usize) -> bool {
    if offset + magic.len() > bytes.len() {
        return false;
    }

    &bytes[offset..offset + magic.len()] == magic
}

fn find_all(bytes: &[u8], magic: &[u8]) -> Vec<usize> {
    if magic.is_empty() || bytes.len() < magic.len() {
        return Vec::new();
    }

    bytes
        .windows(magic.len())
        .enumerate()
        .filter_map(|(index, window)| (window == magic).then_some(index))
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn estimate_size_to_end(bytes: &[u8], offset: usize) -> usize {
    bytes.len().saturating_sub(offset)
}

fn external_workspace(tool: &str, offset: usize) -> std::path::PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    env::temp_dir().join(format!(
        "drill-{tool}-{}-{offset}-{timestamp}",
        std::process::id()
    ))
}

fn format_adapter_error(err: &AdapterError) -> String {
    match err {
        AdapterError::Io(err) => err.to_string(),
        AdapterError::ToolFailed {
            command,
            status_code,
            stderr,
        } => format!("{command} failed with status {status_code:?}: {stderr}"),
        AdapterError::ToolNotFound(tool) => {
            if let Some(rule) = tool_rule(tool) {
                format!(
                    "{tool} was not found in PATH; install package {} ({})",
                    rule.arch_package, rule.install_hint
                )
            } else {
                format!("{tool} was not found in PATH")
            }
        }
        AdapterError::Unsupported(message) => message.clone(),
    }
}

fn confidence_level(valid: bool, score: u8) -> &'static str {
    if valid && score >= 85 {
        "high"
    } else if valid && score >= 60 {
        "medium"
    } else if valid {
        "low"
    } else {
        "unconfirmed"
    }
}
