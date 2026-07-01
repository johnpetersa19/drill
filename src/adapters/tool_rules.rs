#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Extractor,
    Analyzer,
    Converter,
    Flasher,
    Gui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Directory,
    Json,
    Text,
    Binary,
    ToolProject,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolRule {
    pub id: &'static str,
    pub command: &'static str,
    pub aliases: &'static [&'static str],
    pub kind: ToolKind,
    pub arch_package: &'static str,
    pub install_hint: &'static str,
    pub input_rule: &'static str,
    pub output_rule: &'static str,
    pub extract_all_args: &'static [&'static str],
    pub quiet_or_headless_args: &'static [&'static str],
    pub success_rule: &'static str,
    pub output_format: OutputFormat,
    pub notes: &'static str,
}

pub const BINWALK: ToolRule = ToolRule {
    id: "binwalk",
    command: "binwalk",
    aliases: &["binwalk"],
    kind: ToolKind::Extractor,
    arch_package: "binwalk",
    install_hint: "pacman -S binwalk",
    input_rule: "Input file is the final positional FILE_NAME argument.",
    output_rule:
        "Use --directory <DIRECTORY>; extraction defaults to a directory named extractions.",
    extract_all_args: &["--extract", "--matryoshka"],
    quiet_or_headless_args: &["--quiet"],
    success_rule: "Exit status 0 means the command completed; non-zero is an adapter error.",
    output_format: OutputFormat::Directory,
    notes: "Use --log <LOG> when JSON scan results are needed.",
};

pub const UEFIEXTRACT: ToolRule = ToolRule {
    id: "uefiextract",
    command: "uefiextract",
    aliases: &["UEFIExtract", "uefiextract"],
    kind: ToolKind::Extractor,
    arch_package: "uefitool-bin",
    install_hint: "install uefitool-bin (AUR on many Arch setups)",
    input_rule: "Input file is the first positional imagefile argument.",
    output_rule: "Creates an imagefile-derived .dump directory next to the input file.",
    extract_all_args: &["all"],
    quiet_or_headless_args: &[],
    success_rule: "Exit status 0 means success for general extraction. GUID mode uses a bit mask.",
    output_format: OutputFormat::Directory,
    notes: "Mode all dumps all tree items. Mode unpack is legacy compatibility. Mode dump skips report and GUID database.",
};

pub const UEFIFIND: ToolRule = ToolRule {
    id: "uefifind",
    command: "uefifind",
    aliases: &["UEFIFind", "uefifind"],
    kind: ToolKind::Analyzer,
    arch_package: "uefitool-bin",
    install_hint: "install uefitool-bin (AUR on many Arch setups)",
    input_rule: "Input file is the first positional imagefile argument.",
    output_rule: "Writes matches to stdout; no output directory is produced.",
    extract_all_args: &[],
    quiet_or_headless_args: &[],
    success_rule: "Exit status 0 means the search command completed.",
    output_format: OutputFormat::Text,
    notes: "Search forms: imagefile {header|body|all} {list|count} pattern, or imagefile file patternsfile.",
};

pub const GHIDRA_ANALYZE_HEADLESS: ToolRule = ToolRule {
    id: "ghidra-analyzeHeadless",
    command: "ghidra-analyzeHeadless",
    aliases: &["ghidra", "analyzeHeadless", "ghidra-analyzeHeadless"],
    kind: ToolKind::Analyzer,
    arch_package: "ghidra",
    install_hint: "pacman -S ghidra",
    input_rule: "Use -import <file-or-directory> after project_location and project_name.",
    output_rule: "Creates or updates a Ghidra project at project_location/project_name unless -deleteProject is used.",
    extract_all_args: &[],
    quiet_or_headless_args: &["-deleteProject"],
    success_rule: "Exit status 0 means headless import/analysis completed; logs may contain analysis warnings.",
    output_format: OutputFormat::ToolProject,
    notes: "Use -noanalysis to import only, -recursive for directory imports, -scriptlog and -log for logs.",
};

pub const IASL: ToolRule = ToolRule {
    id: "iasl",
    command: "iasl",
    aliases: &["iasl"],
    kind: ToolKind::Converter,
    arch_package: "acpica",
    install_hint: "pacman -S acpica",
    input_rule: "Input AML/ASL files are positional file arguments.",
    output_rule: "Use -p <prefix> to control output path and filename prefix.",
    extract_all_args: &[],
    quiet_or_headless_args: &["-vs"],
    success_rule: "Exit status 0 means compile/disassemble completed without fatal errors.",
    output_format: OutputFormat::Text,
    notes: "Use ACPI disassembly options from iasl help when AML table decoding is wired.",
};

pub const FLASHROM: ToolRule = ToolRule {
    id: "flashrom",
    command: "flashrom",
    aliases: &["flashrom"],
    kind: ToolKind::Flasher,
    arch_package: "flashrom",
    install_hint: "pacman -S flashrom",
    input_rule: "For read operations, input is the selected programmer/chip. For verification/write, input is the file argument.",
    output_rule: "Use -r <file> to read flash to a file; use -o <logfile> for logs.",
    extract_all_args: &["-r"],
    quiet_or_headless_args: &[],
    success_rule: "Exit status 0 means the requested flash operation completed. Non-zero must be treated as high-risk failure.",
    output_format: OutputFormat::Binary,
    notes: "Write, erase, and force operations are destructive and must not be run by automatic analysis adapters.",
};

pub const UNSQUASHFS: ToolRule = ToolRule {
    id: "unsquashfs",
    command: "unsquashfs",
    aliases: &["unsquashfs"],
    kind: ToolKind::Extractor,
    arch_package: "squashfs-tools",
    install_hint: "pacman -S squashfs-tools",
    input_rule: "Input filesystem image is the positional FILESYSTEM argument.",
    output_rule: "Use -d <directory> for extraction output when the adapter is expanded.",
    extract_all_args: &[],
    quiet_or_headless_args: &[],
    success_rule: "Exit status 0 means extraction/listing completed.",
    output_format: OutputFormat::Directory,
    notes: "The short help points to -help-section extraction/information/exit for detailed rules.",
};

pub const MKSQUASHFS: ToolRule = ToolRule {
    id: "mksquashfs",
    command: "mksquashfs",
    aliases: &["mksquashfs"],
    kind: ToolKind::Converter,
    arch_package: "squashfs-tools",
    install_hint: "pacman -S squashfs-tools",
    input_rule:
        "Input sources are one or more positional source paths before the output filesystem path.",
    output_rule: "Output filesystem is the positional FILESYSTEM argument after sources.",
    extract_all_args: &[],
    quiet_or_headless_args: &[],
    success_rule: "Exit status 0 means filesystem creation completed.",
    output_format: OutputFormat::Binary,
    notes: "Mostly a packer, not an extractor. Use only when Drill adds repacking support.",
};

pub const SEVEN_ZIP: ToolRule = ToolRule {
    id: "7z",
    command: "7z",
    aliases: &["7z", "7za", "7zr"],
    kind: ToolKind::Extractor,
    arch_package: "7zip",
    install_hint: "pacman -S 7zip",
    input_rule: "Archive is the <archive_name> after command x/e/l/t.",
    output_rule: "Use -o<Directory> with command x to extract with full paths.",
    extract_all_args: &["x", "-y"],
    quiet_or_headless_args: &["-bd", "-bb0"],
    success_rule:
        "Exit status 0 means success; non-zero means warning/error and should be captured.",
    output_format: OutputFormat::Directory,
    notes: "Use command l for listing, t for integrity testing, x for extraction with full paths.",
};

pub const FFPROBE: ToolRule = ToolRule {
    id: "ffprobe",
    command: "ffprobe",
    aliases: &["ffprobe"],
    kind: ToolKind::Analyzer,
    arch_package: "ffmpeg",
    install_hint: "pacman -S ffmpeg",
    input_rule: "Input media is the final INPUT_FILE argument.",
    output_rule: "Use -output_format json with -show_format and -show_streams for machine-readable output.",
    extract_all_args: &[],
    quiet_or_headless_args: &["-hide_banner", "-v", "error"],
    success_rule: "Exit status 0 means probing completed; use -show_error for structured probe errors.",
    output_format: OutputFormat::Json,
    notes: "Preferred metadata command: ffprobe -hide_banner -v error -output_format json -show_format -show_streams INPUT.",
};

pub const FFMPEG: ToolRule = ToolRule {
    id: "ffmpeg",
    command: "ffmpeg",
    aliases: &["ffmpeg"],
    kind: ToolKind::Converter,
    arch_package: "ffmpeg",
    install_hint: "pacman -S ffmpeg",
    input_rule: "Use -i <input> as the input file argument.",
    output_rule: "Output path is the final output file argument.",
    extract_all_args: &[],
    quiet_or_headless_args: &["-hide_banner", "-v", "error", "-y"],
    success_rule: "Exit status 0 means conversion/extraction completed.",
    output_format: OutputFormat::Binary,
    notes: "For Drill analysis, prefer ffprobe for metadata; use ffmpeg only for explicit media extraction/conversion.",
};

pub const UEFITOOL: ToolRule = ToolRule {
    id: "uefitool",
    command: "uefitool",
    aliases: &["UEFITool", "uefitool"],
    kind: ToolKind::Gui,
    arch_package: "uefitool-bin",
    install_hint: "install uefitool-bin (AUR on many Arch setups)",
    input_rule: "GUI command in the Arch uefitool-bin package; not suitable for automatic headless extraction.",
    output_rule: "No stable headless output rule for this package command.",
    extract_all_args: &[],
    quiet_or_headless_args: &[],
    success_rule: "Do not use as an automatic adapter. Use uefiextract for headless extraction.",
    output_format: OutputFormat::None,
    notes: "Registered only so the database can reject or redirect GUI UEFITool usage cleanly.",
};

pub const ALL_TOOL_RULES: &[ToolRule] = &[
    BINWALK,
    UEFIEXTRACT,
    UEFIFIND,
    GHIDRA_ANALYZE_HEADLESS,
    IASL,
    FLASHROM,
    UNSQUASHFS,
    MKSQUASHFS,
    SEVEN_ZIP,
    FFPROBE,
    FFMPEG,
    UEFITOOL,
];

pub fn tool_rule(id_or_alias: &str) -> Option<&'static ToolRule> {
    ALL_TOOL_RULES
        .iter()
        .find(|rule| rule.id == id_or_alias || rule.aliases.contains(&id_or_alias))
}

pub const REQUIRED_ARCH_PACKAGES: &[&str] = &[
    "binwalk",
    "uefitool-bin",
    "ghidra",
    "acpica",
    "flashrom",
    "squashfs-tools",
    "7zip",
    "ffmpeg",
];
