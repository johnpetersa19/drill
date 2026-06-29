pub mod icons {
    pub const CHANGELOG: &str = "text-x-changelog-symbolic";
    pub const COPYING: &str = "text-x-copying-symbolic";
    pub const FOLDER: &str = "folder-symbolic";
    pub const MAKEFILE: &str = "text-makefile-symbolic";
    pub const MAINTAINERS: &str = "text-x-authors-symbolic";
    pub const MARKDOWN: &str = "text-markdown-symbolic";
    pub const README: &str = "text-x-readme-symbolic";
    pub const VCS_GIT: &str = "builder-vcs-git-symbolic";
}

const BUILD_FILE_NAMES: &[&str] = &[
    "build.ninja",
    "build.gradle",
    "cmakelists.txt",
    "containerfile",
    "configure",
    "dockerfile",
    "go.mod",
    "makefile",
    "meson.build",
    "meson.options",
    "package.json",
    "pom.xml",
    "pyproject.toml",
    "requirements.txt",
    "sketch.yaml",
    "sketch.yml",
    "wscript",
];

const FULL_FILENAME_ICONS: &[(&str, &str)] = &[
    (".editorconfig", "format-indent-more-symbolic"),
    (".gitattributes", icons::VCS_GIT),
    (".gitignore", icons::VCS_GIT),
    (".gitmodules", icons::VCS_GIT),
];

pub fn for_path(path_or_name: &str, fallback_icon: &'static str) -> &'static str {
    let name = basename(path_or_name);
    let lower = name.to_ascii_lowercase();

    if is_directory_name(&lower) || fallback_icon == icons::FOLDER {
        return icons::FOLDER;
    }

    if let Some(icon) = builder_prefix_icon(name) {
        return icon;
    }

    if let Some((_, icon)) = FULL_FILENAME_ICONS
        .iter()
        .find(|(filename, _)| lower == *filename)
    {
        return icon;
    }

    if BUILD_FILE_NAMES.contains(&lower.as_str()) {
        return icons::MAKEFILE;
    }

    extension(&lower)
        .and_then(builder_content_icon)
        .unwrap_or(fallback_icon)
}

fn basename(path_or_name: &str) -> &str {
    path_or_name.rsplit('/').next().unwrap_or(path_or_name)
}

fn extension(lower_name: &str) -> Option<&str> {
    lower_name.rsplit_once('.').map(|(_, ext)| ext)
}

fn builder_prefix_icon(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();

    if lower.starts_with("readme") {
        Some(icons::README)
    } else if lower.starts_with("news") || lower.starts_with("changelog") {
        Some(icons::CHANGELOG)
    } else if lower.starts_with("copying") || lower.starts_with("license") {
        Some(icons::COPYING)
    } else if lower.starts_with("authors") || lower.starts_with("maintainers") {
        Some(icons::MAINTAINERS)
    } else if lower.starts_with("dockerfile") || lower.starts_with("containerfile") {
        Some(icons::MAKEFILE)
    } else {
        None
    }
}

fn builder_content_icon(ext: &str) -> Option<&'static str> {
    match ext {
        "blp" => Some("text-x-blueprint-symbolic"),
        "c" => Some("text-x-csrc-symbolic"),
        "cc" | "cpp" | "cxx" => Some("text-x-c++src-symbolic"),
        "cmake" => Some(icons::MAKEFILE),
        "css" | "scss" => Some("text-css-symbolic"),
        "go" => Some("text-x-go-symbolic"),
        "h" => Some("text-x-chdr-symbolic"),
        "hpp" | "hxx" => Some("text-x-c++src-symbolic"),
        "html" | "htm" => Some("text-html-symbolic"),
        "ino" => Some("text-arduino-symbolic"),
        "js" | "jsx" | "json" | "ts" | "tsx" => Some("text-x-javascript-symbolic"),
        "md" | "markdown" => Some(icons::MARKDOWN),
        "php" => Some("application-x-php-symbolic"),
        "py" | "py3" | "pyw" => Some("text-x-python-symbolic"),
        "rb" => Some("text-x-ruby-symbolic"),
        "rs" => Some("text-rust-symbolic"),
        "sh" | "bash" | "zsh" | "fish" => Some("text-x-script-symbolic"),
        "sql" => Some("text-sql-symbolic"),
        "swift" => Some("text-swift-symbolic"),
        "ui" | "xml" => Some("text-xml-symbolic"),
        "vala" | "vapi" => Some("text-x-vala-symbolic"),
        _ => None,
    }
}

fn is_directory_name(lower_name: &str) -> bool {
    matches!(
        lower_name,
        "build"
            | "builddir"
            | "cargo-home"
            | "data"
            | "debug"
            | "deps"
            | "dist"
            | "icons"
            | "images"
            | "meson-info"
            | "meson-logs"
            | "meson-private"
            | "node_modules"
            | "po"
            | "registry"
            | "release"
            | "src"
            | "target"
            | "vendor"
    )
}
