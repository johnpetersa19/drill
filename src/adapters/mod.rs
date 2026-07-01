#![allow(dead_code)]

pub mod tool_rules;

use self::tool_rules::{
    tool_rule, BINWALK, FFMPEG, FFPROBE, GHIDRA_ANALYZE_HEADLESS, IASL, SEVEN_ZIP, UEFIEXTRACT,
    UNSQUASHFS,
};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug)]
pub enum AdapterError {
    Io(io::Error),
    ToolFailed {
        command: String,
        status_code: Option<i32>,
        stderr: String,
    },
    ToolNotFound(String),
    Unsupported(String),
}

impl From<io::Error> for AdapterError {
    fn from(err: io::Error) -> Self {
        AdapterError::Io(err)
    }
}

#[derive(Debug, Clone)]
pub struct ExternalAnalysis {
    pub command: String,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub output_dir: Option<PathBuf>,
    pub artifacts: Vec<ExternalArtifact>,
}

#[derive(Debug, Clone)]
pub struct ExternalArtifact {
    pub name: String,
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

pub trait ExternalAdapter {
    fn name(&self) -> &'static str;
    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError>;
}

pub struct UefiExtractAdapter;
pub struct UefiToolAdapter;
pub struct UefiFindAdapter;
pub struct BinwalkAdapter;
pub struct GhidraAdapter;
pub struct IaslAdapter;
pub struct FlashromAdapter;
pub struct UnsquashfsAdapter;
pub struct MksquashfsAdapter;
pub struct SevenZipAdapter;
pub struct FfprobeAdapter;
pub struct FfmpegAdapter;

impl ExternalAdapter for UefiToolAdapter {
    fn name(&self) -> &'static str {
        "uefitool"
    }

    fn analyze(&self, _input: &Path, _workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        Err(AdapterError::Unsupported(
            tool_rule("uefitool")
                .map(|rule| rule.success_rule)
                .unwrap_or("uefitool is not supported for automatic headless extraction")
                .to_string(),
        ))
    }
}

impl ExternalAdapter for UefiExtractAdapter {
    fn name(&self) -> &'static str {
        "uefiextract"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output = run_command(
            UEFIEXTRACT.command,
            [
                input.as_os_str(),
                OsStr::new(UEFIEXTRACT.extract_all_args[0]),
            ],
        )?;
        let dump_dir = input.with_extension("dump");

        Ok(ExternalAnalysis {
            command: format!(
                "{} {} {}",
                UEFIEXTRACT.command,
                input.display(),
                UEFIEXTRACT.extract_all_args[0]
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: dump_dir.exists().then_some(dump_dir.clone()),
            artifacts: collect_artifacts(&dump_dir, workspace)?,
        })
    }
}

impl ExternalAdapter for UefiFindAdapter {
    fn name(&self) -> &'static str {
        "uefifind"
    }

    fn analyze(&self, _input: &Path, _workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        Err(AdapterError::Unsupported(
            "uefifind requires a search scope, mode, and pattern; wire it through a detector-specific rule before automatic use"
                .to_string(),
        ))
    }
}

impl ExternalAdapter for BinwalkAdapter {
    fn name(&self) -> &'static str {
        "binwalk"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output_dir = workspace.join("binwalk");
        fs::create_dir_all(&output_dir)?;

        let output = run_command(
            BINWALK.command,
            [
                OsStr::new(BINWALK.extract_all_args[0]),
                OsStr::new(BINWALK.extract_all_args[1]),
                OsStr::new(BINWALK.quiet_or_headless_args[0]),
                OsStr::new("--directory"),
                output_dir.as_os_str(),
                input.as_os_str(),
            ],
        )?;

        Ok(ExternalAnalysis {
            command: format!(
                "{} {} {} {} --directory {} {}",
                BINWALK.command,
                BINWALK.extract_all_args[0],
                BINWALK.extract_all_args[1],
                BINWALK.quiet_or_headless_args[0],
                output_dir.display(),
                input.display()
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: Some(output_dir.clone()),
            artifacts: collect_artifacts(&output_dir, workspace)?,
        })
    }
}

impl ExternalAdapter for GhidraAdapter {
    fn name(&self) -> &'static str {
        "ghidra"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let project_dir = workspace.join("ghidra_project");
        fs::create_dir_all(&project_dir)?;

        let output = run_command(
            GHIDRA_ANALYZE_HEADLESS.command,
            [
                project_dir.as_os_str(),
                OsStr::new("drill"),
                OsStr::new("-import"),
                input.as_os_str(),
                OsStr::new(GHIDRA_ANALYZE_HEADLESS.quiet_or_headless_args[0]),
            ],
        )?;

        Ok(ExternalAnalysis {
            command: format!(
                "{} {} drill -import {} {}",
                GHIDRA_ANALYZE_HEADLESS.command,
                project_dir.display(),
                input.display(),
                GHIDRA_ANALYZE_HEADLESS.quiet_or_headless_args[0]
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: Some(project_dir.clone()),
            artifacts: Vec::new(),
        })
    }
}

impl ExternalAdapter for IaslAdapter {
    fn name(&self) -> &'static str {
        "iasl"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output_prefix = workspace.join("iasl").join("table");
        if let Some(parent) = output_prefix.parent() {
            fs::create_dir_all(parent)?;
        }

        let output = run_command(
            IASL.command,
            [
                OsStr::new(IASL.quiet_or_headless_args[0]),
                OsStr::new("-d"),
                OsStr::new("-p"),
                output_prefix.as_os_str(),
                input.as_os_str(),
            ],
        )?;

        let output_dir = output_prefix
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| workspace.to_path_buf());

        Ok(ExternalAnalysis {
            command: format!(
                "{} {} -d -p {} {}",
                IASL.command,
                IASL.quiet_or_headless_args[0],
                output_prefix.display(),
                input.display()
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: Some(output_dir.clone()),
            artifacts: collect_artifacts(&output_dir, workspace)?,
        })
    }
}

impl ExternalAdapter for FlashromAdapter {
    fn name(&self) -> &'static str {
        "flashrom"
    }

    fn analyze(&self, _input: &Path, _workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        Err(AdapterError::Unsupported(
            "flashrom is hardware-facing and can read/write real flash chips; automatic analysis adapters must not run it"
                .to_string(),
        ))
    }
}

impl ExternalAdapter for UnsquashfsAdapter {
    fn name(&self) -> &'static str {
        "unsquashfs"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output_dir = workspace.join("unsquashfs");
        fs::create_dir_all(&output_dir)?;

        let output = run_command(
            UNSQUASHFS.command,
            [OsStr::new("-d"), output_dir.as_os_str(), input.as_os_str()],
        )?;

        Ok(ExternalAnalysis {
            command: format!(
                "{} -d {} {}",
                UNSQUASHFS.command,
                output_dir.display(),
                input.display()
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: Some(output_dir.clone()),
            artifacts: collect_artifacts(&output_dir, workspace)?,
        })
    }
}

impl ExternalAdapter for MksquashfsAdapter {
    fn name(&self) -> &'static str {
        "mksquashfs"
    }

    fn analyze(&self, _input: &Path, _workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        Err(AdapterError::Unsupported(
            "mksquashfs creates filesystem images; it is a packer adapter, not an automatic analysis extractor"
                .to_string(),
        ))
    }
}

impl ExternalAdapter for SevenZipAdapter {
    fn name(&self) -> &'static str {
        "7z"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output_dir = workspace.join("7z");
        fs::create_dir_all(&output_dir)?;
        let output_switch = format!("-o{}", output_dir.display());

        let output = run_command(
            SEVEN_ZIP.command,
            [
                OsStr::new(SEVEN_ZIP.extract_all_args[0]),
                OsStr::new(SEVEN_ZIP.extract_all_args[1]),
                OsStr::new(SEVEN_ZIP.quiet_or_headless_args[0]),
                OsStr::new(SEVEN_ZIP.quiet_or_headless_args[1]),
                OsStr::new(&output_switch),
                input.as_os_str(),
            ],
        )?;

        Ok(ExternalAnalysis {
            command: format!(
                "{} {} {} {} {} {} {}",
                SEVEN_ZIP.command,
                SEVEN_ZIP.extract_all_args[0],
                SEVEN_ZIP.extract_all_args[1],
                SEVEN_ZIP.quiet_or_headless_args[0],
                SEVEN_ZIP.quiet_or_headless_args[1],
                output_switch,
                input.display()
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: Some(output_dir.clone()),
            artifacts: collect_artifacts(&output_dir, workspace)?,
        })
    }
}

impl ExternalAdapter for FfprobeAdapter {
    fn name(&self) -> &'static str {
        "ffprobe"
    }

    fn analyze(&self, input: &Path, _workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output = run_command(
            FFPROBE.command,
            [
                OsStr::new(FFPROBE.quiet_or_headless_args[0]),
                OsStr::new(FFPROBE.quiet_or_headless_args[1]),
                OsStr::new(FFPROBE.quiet_or_headless_args[2]),
                OsStr::new("-output_format"),
                OsStr::new("json"),
                OsStr::new("-show_format"),
                OsStr::new("-show_streams"),
                input.as_os_str(),
            ],
        )?;

        Ok(ExternalAnalysis {
            command: format!(
                "{} -hide_banner -v error -output_format json -show_format -show_streams {}",
                FFPROBE.command,
                input.display()
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: None,
            artifacts: Vec::new(),
        })
    }
}

impl ExternalAdapter for FfmpegAdapter {
    fn name(&self) -> &'static str {
        "ffmpeg"
    }

    fn analyze(&self, input: &Path, workspace: &Path) -> Result<ExternalAnalysis, AdapterError> {
        let output_file = workspace.join("ffmetadata.txt");

        let output = run_command(
            FFMPEG.command,
            [
                OsStr::new(FFMPEG.quiet_or_headless_args[0]),
                OsStr::new(FFMPEG.quiet_or_headless_args[1]),
                OsStr::new(FFMPEG.quiet_or_headless_args[2]),
                OsStr::new(FFMPEG.quiet_or_headless_args[3]),
                OsStr::new("-i"),
                input.as_os_str(),
                OsStr::new("-f"),
                OsStr::new("ffmetadata"),
                output_file.as_os_str(),
            ],
        )?;

        let artifacts = if output_file.exists() {
            vec![ExternalArtifact {
                name: "ffmetadata.txt".to_string(),
                path: output_file.clone(),
                bytes: fs::read(&output_file)?,
            }]
        } else {
            Vec::new()
        };

        Ok(ExternalAnalysis {
            command: format!(
                "{} -hide_banner -v error -y -i {} -f ffmetadata {}",
                FFMPEG.command,
                input.display(),
                output_file.display()
            ),
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_dir: Some(workspace.to_path_buf()),
            artifacts,
        })
    }
}

pub fn adapter_for(tool: &str) -> Option<Box<dyn ExternalAdapter>> {
    match tool {
        "uefitool" | "UEFITool" => Some(Box::new(UefiToolAdapter)),
        "uefiextract" | "UEFIExtract" => Some(Box::new(UefiExtractAdapter)),
        "uefifind" | "UEFIFind" => Some(Box::new(UefiFindAdapter)),
        "binwalk" => Some(Box::new(BinwalkAdapter)),
        "ghidra" | "analyzeHeadless" | "ghidra-analyzeHeadless" => Some(Box::new(GhidraAdapter)),
        "iasl" => Some(Box::new(IaslAdapter)),
        "flashrom" => Some(Box::new(FlashromAdapter)),
        "unsquashfs" => Some(Box::new(UnsquashfsAdapter)),
        "mksquashfs" => Some(Box::new(MksquashfsAdapter)),
        "7z" | "7za" | "7zr" => Some(Box::new(SevenZipAdapter)),
        "ffprobe" => Some(Box::new(FfprobeAdapter)),
        "ffmpeg" => Some(Box::new(FfmpegAdapter)),
        _ => None,
    }
}

fn run_command<I, S>(program: &str, args: I) -> Result<Output, AdapterError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => AdapterError::ToolNotFound(program.to_string()),
            _ => AdapterError::Io(err),
        })?;

    if !output.status.success() {
        return Err(AdapterError::ToolFailed {
            command: program.to_string(),
            status_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(output)
}

fn collect_artifacts(root: &Path, workspace: &Path) -> Result<Vec<ExternalArtifact>, AdapterError> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut artifacts = Vec::new();
    collect_artifacts_recursive(root, root, workspace, &mut artifacts)?;
    Ok(artifacts)
}

fn collect_artifacts_recursive(
    root: &Path,
    current: &Path,
    workspace: &Path,
    artifacts: &mut Vec<ExternalArtifact>,
) -> Result<(), AdapterError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_artifacts_recursive(root, &path, workspace, artifacts)?;
        } else if path.is_file() && !path.starts_with(workspace.join("input.bin")) {
            let bytes = fs::read(&path)?;
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            artifacts.push(ExternalArtifact { name, path, bytes });
        }
    }

    Ok(())
}
