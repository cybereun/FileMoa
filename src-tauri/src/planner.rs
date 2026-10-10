use crate::progress::Ctl;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

/// One of these alone means "this folder is a repository".
const STRONG_PROJECT_MARKERS: &[&str] = &[".git", ".hg", ".svn"];

/// Build/manifest files. A single one is common in a Downloads folder, so it
/// only counts together with a second marker or a source folder.
const PROJECT_FILE_MARKERS: &[&str] = &[
    "package.json",
    "requirements.txt",
    "pyproject.toml",
    "cargo.toml",
    "pom.xml",
    "build.gradle",
    "composer.json",
    "go.mod",
    "cmakelists.txt",
    "gemfile",
    "pubspec.yaml",
    "deno.json",
    "makefile",
];

/// Project files recognised by extension (Visual Studio and friends).
const PROJECT_EXTENSIONS: &[&str] = &["sln", "csproj", "vbproj", "fsproj"];

/// Folders that, next to one manifest file, make a project certain.
const PROJECT_SOURCE_DIRS: &[&str] = &[
    "src",
    "lib",
    "node_modules",
    "target",
    "venv",
    ".venv",
    "vendor",
];

const TEMP_SUFFIXES: &[&str] = &[".crdownload", ".part", ".tmp", ".download"];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCategory {
    pub name: String,
    pub extensions: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRule {
    pub keyword: String,
    pub category: String,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest {
    pub root_path: String,
    pub include_subfolders: bool,
    #[serde(default)]
    pub custom_categories: Vec<CustomCategory>,
    /// `ko` or `en`. The core remains language-aware so folder names match the UI.
    #[serde(default = "default_language")]
    pub language: String,
    /// `type`, `date` (year/month), `date_day` (year/month/day), `year`, `size`, or `name`.
    #[serde(default = "default_basis")]
    pub basis: String,
    /// `safe`, `normal`, or `strong`. All modes still require preview/approval.
    #[serde(default = "default_mode")]
    pub mode: String,
    /// `created`, `modified`, or `accessed` when `basis` is date/date_day/year.
    #[serde(default = "default_date_basis")]
    pub date_basis: String,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default)]
    pub excluded_paths: Vec<String>,
    #[serde(default)]
    pub excluded_extensions: Vec<String>,
    #[serde(default)]
    pub keyword_rules: Vec<KeywordRule>,
}

fn default_language() -> String {
    "ko".into()
}

fn default_basis() -> String {
    "type".into()
}

fn default_date_basis() -> String {
    "modified".into()
}

fn default_mode() -> String {
    "safe".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAction {
    pub source_path: String,
    pub destination_path: String,
    pub category: String,
    pub reason: String,
    pub status: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub modified_unix_secs: Option<u64>,
    #[serde(default)]
    pub collision: bool,
    /// Folders this move had to create (shallowest first). Undo removes the
    /// ones that are empty again; folders that already existed are never listed.
    #[serde(default)]
    pub created_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlan {
    pub root_path: String,
    pub actions: Vec<PlannedAction>,
    pub exclusions: Vec<String>,
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub basis: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub root_paths: Vec<String>,
    /// Identifies the persisted run record/journal; empty until execution starts.
    #[serde(default)]
    pub run_id: String,
    /// Handle to the plan kept by the backend; the UI sends only this to execute.
    #[serde(default)]
    pub plan_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub hash: String,
    pub size_bytes: u64,
    pub file_paths: Vec<String>,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFile {
    pub path: String,
    pub size_bytes: u64,
}

/// Scan output plus how many cloud-only files were left alone. Reading such a
/// file (hashing) makes OneDrive/Drive download it, and listing it as "large"
/// is misleading because it occupies no local space.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult<T> {
    pub items: Vec<T>,
    pub cloud_only_skipped: usize,
}

const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x4_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

/// Placeholder/offline attributes set by cloud sync providers for files whose
/// content is not stored locally.
#[cfg_attr(not(windows), allow(dead_code))]
fn attributes_mean_cloud_only(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

fn is_cloud_only(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        attributes_mean_cloud_only(metadata.file_attributes())
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

fn is_english(language: &str) -> bool {
    language.eq_ignore_ascii_case("en")
}

fn localised(language: &str, ko: &'static str, en: &'static str) -> &'static str {
    if is_english(language) {
        en
    } else {
        ko
    }
}

fn builtin(ext: &str, language: &str, file_name: &str) -> (&'static str, &'static str) {
    let lower_name = file_name.to_ascii_lowercase();
    let screenshot = ["screenshot", "snippingtool", "capture", "스크린샷", "캡처"]
        .iter()
        .any(|keyword| lower_name.contains(&keyword.to_ascii_lowercase()));
    match (ext, screenshot) {
        ("pdf", _) => (
            localised(language, "문서/PDF", "Documents/PDF"),
            localised(language, "PDF 문서", "PDF document"),
        ),
        ("doc" | "docx", _) => (
            localised(language, "문서/Word", "Documents/Word"),
            localised(language, "Word 문서", "Word document"),
        ),
        ("xls" | "xlsx" | "csv", _) => (
            localised(language, "문서/Excel", "Documents/Excel"),
            localised(language, "스프레드시트", "Spreadsheet"),
        ),
        ("ppt" | "pptx", _) => (
            localised(language, "문서/PowerPoint", "Documents/PowerPoint"),
            localised(language, "프레젠테이션", "Presentation"),
        ),
        ("hwp" | "hwpx", _) => (
            localised(language, "문서/한글", "Documents/Hangul"),
            localised(language, "한글 문서", "Hangul document"),
        ),
        ("txt" | "md" | "rtf", _) => (
            localised(language, "문서/텍스트", "Documents/Text"),
            localised(language, "텍스트 문서", "Text document"),
        ),
        ("epub" | "mobi" | "azw" | "azw3", _) => (
            localised(language, "문서/전자책", "Documents/Ebooks"),
            localised(language, "전자책", "Ebook"),
        ),
        (_, true) => (
            localised(language, "이미지/스크린샷", "Images/Screenshots"),
            localised(language, "스크린샷", "Screenshot"),
        ),
        (
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "heic" | "avif" | "svg"
            | "raw" | "cr2" | "nef" | "arw",
            _,
        ) => (
            localised(language, "이미지", "Images"),
            localised(language, "이미지", "Image"),
        ),
        ("mp4" | "mov" | "avi" | "mkv" | "wmv" | "webm" | "m4v" | "mts", _) => (
            localised(language, "동영상", "Videos"),
            localised(language, "동영상", "Video"),
        ),
        ("mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "wma", _) => (
            localised(language, "오디오", "Audio"),
            localised(language, "오디오", "Audio"),
        ),
        ("zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz", _) => (
            localised(language, "압축파일", "Archives"),
            localised(language, "압축 파일", "Archive"),
        ),
        ("exe" | "msi" | "msix" | "msixbundle" | "appx" | "appxbundle", _) => (
            localised(language, "설치파일", "Installers"),
            localised(language, "설치 파일", "Installer"),
        ),
        (
            "js" | "jsx" | "ts" | "tsx" | "html" | "css" | "scss" | "py" | "java" | "kt" | "cpp"
            | "c" | "cs" | "go" | "rs" | "php" | "sql" | "json" | "yaml" | "yml" | "toml" | "xml"
            | "env" | "ipynb",
            _,
        ) => (
            localised(language, "개발", "Development"),
            localised(language, "개발 파일", "Development file"),
        ),
        ("ttf" | "otf" | "woff" | "woff2", _) => (
            localised(language, "폰트", "Fonts"),
            localised(language, "폰트", "Font"),
        ),
        ("iso" | "img" | "vhd" | "vhdx", _) => (
            localised(language, "디스크 이미지", "Disk images"),
            localised(language, "디스크 이미지", "Disk image"),
        ),
        ("sqlite" | "db" | "parquet", _) => (
            localised(language, "데이터", "Data"),
            localised(language, "데이터 파일", "Data file"),
        ),
        _ => (
            localised(language, "기타", "Other"),
            localised(language, "기타 파일", "Other file"),
        ),
    }
}

pub(crate) fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn path_is_under(path: &Path, parent: &Path) -> bool {
    let path = path_key(path);
    let parent = path_key(parent);
    path == parent || path.starts_with(&(parent + "\\"))
}

/// Resolve 8.3 short names (`PROGRA~1`), junctions and symlinks so a protected
/// folder cannot be reached under another spelling. Falls back to the input
/// when the path does not exist.
fn resolved(path: &Path) -> PathBuf {
    let Ok(real) = fs::canonicalize(path) else {
        return path.to_path_buf();
    };
    let text = real.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        real
    }
}

/// Per-user and system folders from the environment, in resolved form.
fn protected_environment_dirs() -> &'static [PathBuf] {
    static DIRS: std::sync::OnceLock<Vec<PathBuf>> = std::sync::OnceLock::new();
    DIRS.get_or_init(|| {
        [
            "APPDATA",
            "LOCALAPPDATA",
            "SystemRoot",
            "windir",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramData",
        ]
        .iter()
        .filter_map(std::env::var_os)
        .map(|value| resolved(Path::new(&value)))
        .collect()
    })
}

fn protected_spelling(path: &Path) -> bool {
    let key = path_key(path);
    let component_match = |name: &str| {
        path.components().any(|component| {
            component
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        })
    };
    let system_root = key.ends_with(":\\windows") || key.contains(":\\windows\\");
    let program_root = key.ends_with(":\\program files")
        || key.contains(":\\program files\\")
        || key.ends_with(":\\program files (x86)")
        || key.contains(":\\program files (x86)\\")
        || key.ends_with(":\\programdata")
        || key.contains(":\\programdata\\");
    let environment = protected_environment_dirs()
        .iter()
        .any(|dir| path_is_under(path, dir));
    system_root
        || program_root
        || environment
        || component_match(".git")
        || component_match("node_modules")
}

/// Cheap check used inside scan loops over many files.
fn protected(path: &Path) -> bool {
    protected_spelling(path)
}

/// Shared safety check for commands that operate on paths chosen in the UI.
/// Also checks the resolved path, so short names and junctions do not bypass it.
pub fn is_protected_path(path: &Path) -> bool {
    protected_spelling(path) || protected_spelling(&resolved(path))
}

/// Top-level folders that hold system state on every drive.
const PROTECTED_DRIVE_FOLDERS: &[&str] = &["$recycle.bin", "system volume information"];

/// Top-level folders that are only system state on the Windows drive; on a
/// data drive a folder called `Recovery` is usually the user's own.
const PROTECTED_SYSTEM_DRIVE_FOLDERS: &[&str] = &[
    "recovery",
    "windows.old",
    "boot",
    "msocache",
    "$windows.~bt",
    "$windows.~ws",
];

fn on_system_drive(path: &Path) -> bool {
    let drive = path_key(path).chars().take(2).collect::<String>();
    std::env::var("SystemDrive")
        .map(|system| system.to_ascii_lowercase() == drive)
        .unwrap_or(false)
}

/// Why `root` may not be organised, if it may not. Checked on the resolved
/// path: a drive root, the user profile or one of its parents, a system
/// folder, or a protected location.
pub(crate) fn root_rejection(root: &Path) -> Option<&'static str> {
    let real = resolved(root);
    for path in [root, real.as_path()] {
        if path.parent().is_none() {
            return Some("드라이브 루트는 정리할 수 없습니다. 하위 폴더를 선택하세요. / A drive root cannot be organised; pick a folder inside it.");
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            let profile = resolved(Path::new(&profile));
            if path_is_under(&profile, path) {
                return Some("사용자 폴더 전체(또는 그 상위 폴더)는 정리할 수 없습니다. 문서·다운로드 같은 하위 폴더를 선택하세요. / The whole user profile (or a parent of it) cannot be organised; pick a folder inside it.");
            }
        }
        let first = path
            .components()
            .filter_map(|component| match component {
                std::path::Component::Normal(name) => Some(name.to_string_lossy().to_lowercase()),
                _ => None,
            })
            .next();
        if first.as_deref().is_some_and(|name| {
            PROTECTED_DRIVE_FOLDERS.contains(&name)
                || (PROTECTED_SYSTEM_DRIVE_FOLDERS.contains(&name) && on_system_drive(path))
        }) {
            return Some("시스템 폴더는 정리할 수 없습니다. / System folders cannot be organised.");
        }
    }
    (protected_spelling(root) || protected_spelling(&real)).then_some(
        "보호된 시스템 또는 앱 데이터 경로는 정리할 수 없습니다. / Protected system or app-data path.",
    )
}

fn project(path: &Path) -> bool {
    if STRONG_PROJECT_MARKERS
        .iter()
        .any(|marker| path.join(marker).exists())
    {
        return true;
    }
    let mut markers = PROJECT_FILE_MARKERS
        .iter()
        .filter(|marker| path.join(marker).exists())
        .count();
    if markers < 2 {
        // Directory listing is the expensive part; only needed when undecided.
        markers += fs::read_dir(path)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| {
                                PROJECT_EXTENSIONS
                                    .iter()
                                    .any(|known| extension.eq_ignore_ascii_case(known))
                            })
                    })
                    .count()
            })
            .unwrap_or(0);
    }
    markers >= 2
        || (markers == 1
            && PROJECT_SOURCE_DIRS
                .iter()
                .any(|name| path.join(name).is_dir()))
}

/// `project()` memoised per folder, because a scan asks about the same
/// ancestors for every file beneath them.
#[derive(Default)]
struct ProjectCache(HashMap<PathBuf, bool>);

impl ProjectCache {
    fn is_project(&mut self, dir: &Path) -> bool {
        *self
            .0
            .entry(dir.to_path_buf())
            .or_insert_with(|| project(dir))
    }
}

pub(crate) fn sanitize_category(value: &str, language: &str) -> String {
    let mut category = value
        .trim()
        .replace(['\\', '/'], "-")
        .chars()
        .map(|character| {
            if "<>:\"|?*".contains(character) {
                '-'
            } else {
                character
            }
        })
        .collect::<String>();
    while category.contains("..") {
        category = category.replace("..", "-");
    }
    category = category.trim_matches(['.', ' ']).to_string();
    if category.is_empty() {
        category = localised(language, "기타", "Other").into();
    }
    category.chars().take(80).collect()
}

pub(crate) fn root_for_source(plan: &OrganizationPlan, source: &Path) -> PathBuf {
    plan.root_paths
        .iter()
        .map(PathBuf::from)
        .find(|root| path_is_under(source, root))
        .or_else(|| {
            let fallback = PathBuf::from(&plan.root_path);
            (!plan.root_path.contains('\n') && path_is_under(source, &fallback)).then_some(fallback)
        })
        .or_else(|| source.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from(&plan.root_path))
}

fn category_root(root: &Path, category: &str) -> PathBuf {
    root.join(category.replace('/', "\\"))
}

fn safe_destination(dest: PathBuf) -> (PathBuf, bool) {
    safe_destination_with_reserved(dest, &HashSet::new())
}

fn safe_destination_with_reserved(dest: PathBuf, reserved: &HashSet<String>) -> (PathBuf, bool) {
    if !dest.exists() && !reserved.contains(&path_key(&dest)) {
        return (dest, false);
    }
    let stem = dest
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let ext = dest
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for number in 1..10000 {
        let name = if ext.is_empty() {
            format!("{stem} ({number})")
        } else {
            format!("{stem} ({number}).{ext}")
        };
        let candidate = dest.with_file_name(name);
        if !candidate.exists() && !reserved.contains(&path_key(&candidate)) {
            return (candidate, true);
        }
    }
    (dest, true)
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .trim_start_matches('.')
        .to_ascii_lowercase()
}

fn keyword_category(path: &Path, rules: &[KeywordRule]) -> Option<(String, String)> {
    let file_name = path.file_name()?.to_string_lossy();
    rules.iter().find_map(|rule| {
        let keyword = rule.keyword.trim();
        if keyword.is_empty() {
            return None;
        }
        let matched = if rule.case_sensitive {
            file_name.contains(keyword)
        } else {
            file_name
                .to_ascii_lowercase()
                .contains(&keyword.to_ascii_lowercase())
        };
        matched.then(|| {
            (
                rule.category.clone(),
                format!("Filename contains '{keyword}'"),
            )
        })
    })
}

fn custom_category(path: &Path, custom: &[CustomCategory]) -> Option<(String, String)> {
    let ext = extension(path);
    custom.iter().find_map(|rule| {
        let matched = rule.enabled
            && !rule.name.trim().is_empty()
            && rule.extensions.iter().any(|value| {
                value
                    .trim()
                    .trim_start_matches('.')
                    .eq_ignore_ascii_case(&ext)
            });
        matched.then(|| {
            (
                rule.name.clone(),
                "사용자 정의 확장자 규칙 / Custom extension rule".into(),
            )
        })
    })
}

fn metadata_time(metadata: &fs::Metadata, basis: &str) -> Option<DateTime<Local>> {
    let value = match basis.to_ascii_lowercase().as_str() {
        "created" => metadata.created().ok(),
        "accessed" => metadata.accessed().ok(),
        _ => metadata.modified().ok(),
    }?;
    Some(DateTime::<Local>::from(value))
}

fn size_category(size: u64, language: &str) -> String {
    let mb = 1024_u64 * 1024;
    let (label_ko, label_en) = if size <= mb {
        ("초소형", "Tiny")
    } else if size <= 10 * mb {
        ("소형", "Small")
    } else if size <= 100 * mb {
        ("중형", "Medium")
    } else if size <= 1024 * mb {
        ("대형", "Large")
    } else {
        ("초대형", "Huge")
    };
    format!(
        "{}/{}",
        localised(language, "크기", "Size"),
        localised(language, label_ko, label_en)
    )
}

fn category_for(path: &Path, metadata: &fs::Metadata, req: &PlanRequest) -> (String, String) {
    if let Some((category, reason)) = keyword_category(path, &req.keyword_rules) {
        return (sanitize_category(&category, &req.language), reason);
    }
    if req.basis.eq_ignore_ascii_case("date")
        || req.basis.eq_ignore_ascii_case("date_day")
        || req.basis.eq_ignore_ascii_case("year")
    {
        if let Some(date) = metadata_time(metadata, &req.date_basis) {
            let category = if req.basis.eq_ignore_ascii_case("year") {
                date.format("%Y").to_string()
            } else if req.basis.eq_ignore_ascii_case("date_day") {
                date.format("%Y-%m-%d").to_string()
            } else {
                date.format("%Y/%m").to_string()
            };
            return (
                category,
                localised(&req.language, "파일 날짜 기준", "Date-based rule").into(),
            );
        }
    }
    if req.basis.eq_ignore_ascii_case("size") {
        return (
            size_category(metadata.len(), &req.language),
            localised(&req.language, "파일 크기 기준", "Size-based rule").into(),
        );
    }
    if let Some((category, reason)) = custom_category(path, &req.custom_categories) {
        return (sanitize_category(&category, &req.language), reason);
    }
    let file_name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let (category, reason) = builtin(&extension(path), &req.language, &file_name);
    (category.into(), reason.into())
}

fn is_hidden(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if entry
            .metadata()
            .map(|metadata| metadata.file_attributes() & 0x2 != 0)
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn is_temporary_or_hidden(entry: &DirEntry, include_hidden: bool, mode: &str) -> bool {
    let name = entry.file_name().to_string_lossy();
    (!include_hidden && is_hidden(entry))
        || (!mode.eq_ignore_ascii_case("strong")
            && TEMP_SUFFIXES
                .iter()
                .any(|suffix| name.to_ascii_lowercase().ends_with(suffix)))
}

fn excluded_by_user(path: &Path, req: &PlanRequest) -> bool {
    let extension = extension(path);
    req.excluded_extensions.iter().any(|value| {
        value
            .trim()
            .trim_start_matches('.')
            .eq_ignore_ascii_case(&extension)
    }) || req.excluded_paths.iter().any(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return false;
        }
        if Path::new(trimmed).is_absolute() {
            path_is_under(path, Path::new(trimmed))
        } else {
            path.components().any(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(trimmed.trim_matches(['\\', '/']))
            })
        }
    })
}

fn planned_category_roots(root: &Path, req: &PlanRequest) -> Vec<PathBuf> {
    let mut categories: Vec<String> = vec![
        "문서/PDF",
        "문서/Word",
        "문서/Excel",
        "문서/PowerPoint",
        "문서/한글",
        "문서/텍스트",
        "문서/전자책",
        "이미지",
        "이미지/스크린샷",
        "동영상",
        "오디오",
        "압축파일",
        "설치파일",
        "개발",
        "폰트",
        "디스크 이미지",
        "데이터",
        "기타",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    if is_english(&req.language) {
        categories = vec![
            "Documents/PDF",
            "Documents/Word",
            "Documents/Excel",
            "Documents/PowerPoint",
            "Documents/Hangul",
            "Documents/Text",
            "Documents/Ebooks",
            "Images",
            "Images/Screenshots",
            "Videos",
            "Audio",
            "Archives",
            "Installers",
            "Development",
            "Fonts",
            "Disk images",
            "Data",
            "Other",
        ]
        .into_iter()
        .map(String::from)
        .collect();
    }
    categories.extend(
        req.custom_categories
            .iter()
            .filter(|category| category.enabled)
            .map(|category| sanitize_category(&category.name, &req.language)),
    );
    categories.extend(
        req.keyword_rules
            .iter()
            .map(|rule| sanitize_category(&rule.category, &req.language)),
    );
    categories
        .into_iter()
        .map(|category| category_root(root, &category))
        .collect()
}

fn root_warnings(root: &Path) -> Vec<String> {
    let key = path_key(root);
    let mut warnings = Vec::new();
    if key.starts_with("\\\\") {
        warnings.push("네트워크 드라이브입니다. 연결이 끊기면 이동을 중단하고 오류를 기록합니다. / Network location: a disconnect will be reported.".into());
    }
    if key.contains("\\onedrive\\")
        || key.contains("\\google drive\\")
        || key.contains("\\dropbox\\")
    {
        warnings.push("클라우드 동기화 폴더입니다. 온라인 전용 파일은 강제로 다운로드하지 않습니다. / Cloud folder: online-only files are not forced to download.".into());
    }
    if key.len() >= 2 && key.as_bytes()[1] == b':' && !key.starts_with("c:\\") {
        warnings.push("시스템 드라이브가 아닌 위치입니다. 이동 전 저장장치 연결을 확인하세요. / External or removable drive: verify it stays connected.".into());
    }
    warnings
}

#[cfg(test)]
pub fn create_plan(req: PlanRequest) -> Result<OrganizationPlan, String> {
    create_plan_ctl(req, &Ctl::none())
}

pub fn create_plan_ctl(req: PlanRequest, ctl: &Ctl) -> Result<OrganizationPlan, String> {
    let root = PathBuf::from(&req.root_path);
    if !root.is_dir() {
        return Err("선택한 폴더를 찾을 수 없습니다. / The selected folder does not exist.".into());
    }
    if let Some(reason) = root_rejection(&root) {
        return Err(reason.into());
    }
    if project(&root) {
        return Err("개발 프로젝트 폴더는 정리할 수 없습니다. / Development project folders cannot be organised.".into());
    }
    let category_roots = planned_category_roots(&root, &req);
    let mut actions = Vec::new();
    let mut exclusions = Vec::new();
    let mut errors = Vec::new();
    let mut reserved_destinations = HashSet::new();
    let mut projects = ProjectCache::default();
    let warnings = root_warnings(&root);
    let walker = if req.include_subfolders {
        WalkDir::new(&root).min_depth(1)
    } else {
        WalkDir::new(&root).min_depth(1).max_depth(1)
    };
    let mut seen: u64 = 0;
    for item in walker {
        ctl.check()?;
        seen += 1;
        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(format!("스캔 오류 / Scan error: {error}"));
                continue;
            }
        };
        let path = entry.path();
        ctl.report("scan", seen, 0, &path.to_string_lossy(), false);
        if entry.file_type().is_dir() {
            if path != root && projects.is_project(path) {
                exclusions.push(format!(
                    "개발 프로젝트 보호 / Project protected: {}",
                    path.display()
                ));
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if path.ancestors().skip(1).any(|parent| {
            parent != root && path_is_under(parent, &root) && projects.is_project(parent)
        }) {
            continue;
        }
        if category_roots
            .iter()
            .any(|category| path_is_under(path, category))
        {
            exclusions.push(format!(
                "이미 정리된 카테고리 폴더 / Existing category folder: {}",
                path.display()
            ));
            continue;
        }
        if protected(path) || excluded_by_user(path, &req) {
            exclusions.push(format!(
                "사용자/시스템 예외 / User or system exclusion: {}",
                path.display()
            ));
            continue;
        }
        if is_temporary_or_hidden(&entry, req.include_hidden, &req.mode) {
            exclusions.push(format!(
                "숨김 또는 임시 파일 / Hidden or incomplete download: {}",
                path.display()
            ));
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(format!(
                    "파일 정보를 읽을 수 없습니다 / Metadata error ({}): {error}",
                    path.display()
                ));
                continue;
            }
        };
        let (category, rule_reason) = category_for(path, &metadata, &req);
        let base = category_root(&root, &category).join(path.file_name().unwrap_or_default());
        if path_key(path) == path_key(&base) {
            exclusions.push(format!(
                "이미 목적지에 있습니다 / Already organized: {}",
                path.display()
            ));
            continue;
        }
        let (destination, collision) = safe_destination_with_reserved(base, &reserved_destinations);
        reserved_destinations.insert(path_key(&destination));
        let reason = if collision {
            format!("{rule_reason} · 이름 충돌로 번호 추가 / collision-safe rename")
        } else {
            rule_reason
        };
        let modified_unix_secs = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_secs());
        actions.push(PlannedAction {
            source_path: path.display().to_string(),
            destination_path: destination.display().to_string(),
            category,
            reason,
            status: "proposed".into(),
            size_bytes: metadata.len(),
            modified_unix_secs,
            collision,
            created_dirs: Vec::new(),
        });
    }
    Ok(OrganizationPlan {
        root_path: req.root_path,
        actions,
        exclusions,
        errors,
        warnings,
        created_at: Utc::now().to_rfc3339(),
        basis: req.basis,
        language: req.language,
        mode: req.mode,
        root_paths: vec![root.display().to_string()],
        run_id: String::new(),
        plan_id: String::new(),
    })
}

/// Build one dry-run snapshot for several independently selected roots. Each
/// root is scanned with the same settings, while moves never cross root
/// boundaries.
pub fn create_multi_plan_ctl(
    requests: Vec<PlanRequest>,
    ctl: &Ctl,
) -> Result<OrganizationPlan, String> {
    if requests.is_empty() {
        return Err("정리할 폴더를 하나 이상 선택하세요. / Select at least one folder.".into());
    }
    let mut plans = Vec::with_capacity(requests.len());
    for request in requests {
        plans.push(create_plan_ctl(request, ctl)?);
    }
    let language = plans[0].language.clone();
    let basis = plans[0].basis.clone();
    let mode = plans[0].mode.clone();
    let root_paths = plans
        .iter()
        .flat_map(|plan| {
            if plan.root_paths.is_empty() {
                vec![plan.root_path.clone()]
            } else {
                plan.root_paths.clone()
            }
        })
        .collect::<Vec<_>>();
    let mut actions = Vec::new();
    let mut exclusions = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for plan in plans {
        actions.extend(plan.actions);
        exclusions.extend(plan.exclusions);
        errors.extend(plan.errors);
        warnings.extend(plan.warnings);
    }
    Ok(OrganizationPlan {
        root_path: root_paths.join("\n"),
        actions,
        exclusions,
        errors,
        warnings,
        created_at: Utc::now().to_rfc3339(),
        basis,
        language,
        mode,
        root_paths,
        run_id: String::new(),
        plan_id: String::new(),
    })
}

fn plan_roots(plan: &OrganizationPlan) -> Vec<PathBuf> {
    if plan.root_paths.is_empty() {
        plan.root_path.split('\n').map(PathBuf::from).collect()
    } else {
        plan.root_paths.iter().map(PathBuf::from).collect()
    }
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}

/// Why this move may not run, judged only from the plan's own folders: both
/// ends must be absolute, free of `..`, inside the same chosen folder, and
/// outside protected locations.
fn path_blocker(action: &PlannedAction, roots: &[PathBuf]) -> Option<&'static str> {
    let (source, destination) = (
        Path::new(&action.source_path),
        Path::new(&action.destination_path),
    );
    if !source.is_absolute() || !destination.is_absolute() {
        return Some("전체 경로가 아니어서 거부됨 / rejected: not an absolute path");
    }
    if has_parent_component(source) || has_parent_component(destination) {
        return Some("경로에 '..'가 있어 거부됨 / rejected: path contains '..'");
    }
    let Some(root) = roots.iter().find(|root| path_is_under(source, root)) else {
        return Some(
            "원본이 선택한 폴더 밖이라 거부됨 / rejected: source is outside the chosen folder",
        );
    };
    if !path_is_under(destination, root) {
        return Some(
            "대상이 선택한 폴더 밖이라 거부됨 / rejected: destination is outside the chosen folder",
        );
    }
    if protected_spelling(source) || protected_spelling(destination) {
        return Some("보호된 경로라 거부됨 / rejected: protected path");
    }
    None
}

/// Whole seconds since the epoch, the resolution plans and scans compare on.
pub(crate) fn modified_secs(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_secs())
}

/// Called with a plan the backend itself stored: skip files that changed
/// since the preview, so the user approves exactly what is moved.
pub fn mark_changed_since_preview(plan: &mut OrganizationPlan) {
    for action in plan
        .actions
        .iter_mut()
        .filter(|action| action.status == "proposed")
    {
        let Ok(metadata) = fs::metadata(&action.source_path) else {
            continue; // a missing source is reported by the move itself
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_secs());
        let size_changed = metadata.len() != action.size_bytes;
        let time_changed =
            action.modified_unix_secs.is_some() && modified != action.modified_unix_secs;
        if size_changed || time_changed {
            action.status = "skipped".into();
            action.reason =
                "미리보기 이후 파일이 변경되어 건너뜀 / skipped: the file changed after the preview"
                    .into();
        }
    }
}

/// Move `from` to `to` and fail with `AlreadyExists` instead of replacing a
/// file that is already there. The check and the move are one OS call, so a
/// file that appears between planning and moving (cloud sync, a finished
/// download) can never be overwritten. `std::fs::rename` replaces on Windows.
#[cfg(windows)]
fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    let wide = |path: &Path| -> Vec<u16> { path.as_os_str().encode_wide().chain([0]).collect() };
    let (from, to) = (wide(from), wide(to));
    // Flags 0: no MOVEFILE_REPLACE_EXISTING, no copy fallback across volumes.
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn move_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    // hard_link refuses to replace an existing destination.
    fs::hard_link(from, to)?;
    fs::remove_file(from)
}

/// Move without overwriting, picking the next free `name (n).ext` when the
/// destination turns out to be taken. Returns the final path and whether a
/// rename was needed.
fn move_with_unique_name(from: &Path, destination: PathBuf) -> io::Result<(PathBuf, bool)> {
    let mut destination = destination;
    let mut renamed = false;
    for _ in 0..100 {
        match move_no_replace(from, &destination) {
            Ok(()) => return Ok((destination, renamed)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let (next, _) = safe_destination(destination.clone());
                if next == destination {
                    return Err(error);
                }
                destination = next;
                renamed = true;
            }
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free destination name",
    ))
}

/// Run the plan and report every action whose state changed through
/// `on_action` right after it is settled, so callers can journal progress
/// before the next file is touched. If `on_action` fails, no further file is
/// moved: the remaining proposed actions are skipped and the error recorded.
#[cfg(test)]
pub fn execute_plan_with<F>(
    plan: OrganizationPlan,
    on_action: F,
) -> Result<OrganizationPlan, String>
where
    F: FnMut(usize, &PlannedAction) -> Result<(), String>,
{
    execute_plan_ctl(plan, &Ctl::none(), on_action)
}

/// Like `execute_plan_with`, and stops after the file being moved when the
/// user cancels: the rest is skipped and the run stays undoable.
pub fn execute_plan_ctl<F>(
    mut plan: OrganizationPlan,
    ctl: &Ctl,
    mut on_action: F,
) -> Result<OrganizationPlan, String>
where
    F: FnMut(usize, &PlannedAction) -> Result<(), String>,
{
    let roots = plan_roots(&plan);
    let total = plan
        .actions
        .iter()
        .filter(|action| action.status == "proposed")
        .count() as u64;
    let mut done: u64 = 0;
    for index in 0..plan.actions.len() {
        if plan.actions[index].status != "proposed" {
            continue;
        }
        if ctl.cancelled() {
            for action in &mut plan.actions[index..] {
                if action.status == "proposed" {
                    action.status = "skipped".into();
                    action.reason =
                        "사용자가 취소하여 건너뜀 / skipped: cancelled by the user".into();
                }
            }
            break;
        }
        if let Some(reason) = path_blocker(&plan.actions[index], &roots) {
            plan.actions[index].status = "skipped".into();
            plan.actions[index].reason = reason.into();
            plan.errors.push(format!(
                "{}: {}",
                plan.actions[index].source_path, plan.actions[index].reason
            ));
        } else {
            run_action(&mut plan, index);
        }
        done += 1;
        ctl.report(
            "move",
            done,
            total,
            &plan.actions[index].source_path,
            done == total,
        );
        if let Err(error) = on_action(index, &plan.actions[index]) {
            plan.errors.push(error);
            for action in &mut plan.actions[index + 1..] {
                if action.status == "proposed" {
                    action.status = "skipped".into();
                    action.reason =
                        "기록을 쓸 수 없어 중단됨 / stopped: the run record could not be written"
                            .into();
                }
            }
            break;
        }
    }
    Ok(plan)
}

fn run_action(plan: &mut OrganizationPlan, index: usize) {
    let action = &mut plan.actions[index];
    let from_path = PathBuf::from(&action.source_path);
    if !from_path.is_file() {
        action.status = "skipped".into();
        action.reason = "원본 파일이 없어 건너뜀 / source no longer exists".into();
        return;
    }
    let destination = PathBuf::from(&action.destination_path);
    if path_key(&from_path) == path_key(&destination) {
        action.status = "skipped".into();
        return;
    }
    let mut created_dirs = Vec::new();
    if let Some(parent) = destination.parent() {
        created_dirs = missing_ancestors(parent);
        if let Err(error) = fs::create_dir_all(parent) {
            remove_empty_dirs(&created_dirs);
            action.status = "failed".into();
            action.reason =
                format!("폴더를 만들 수 없습니다 / cannot create destination folder: {error}");
            plan.errors
                .push(format!("{}: {}", from_path.display(), action.reason));
            return;
        }
    }
    match move_with_unique_name(&from_path, destination) {
        Ok((destination, collision)) => {
            action.collision = action.collision || collision;
            action.destination_path = destination.display().to_string();
            action.created_dirs = created_dirs
                .iter()
                .map(|dir| dir.display().to_string())
                .collect();
            action.status = "executed".into();
        }
        Err(error) => {
            remove_empty_dirs(&created_dirs);
            action.status = "failed".into();
            action.reason = format!("이동 실패 / move failed: {error}");
            plan.errors
                .push(format!("{}: {}", from_path.display(), action.reason));
        }
    }
}

/// Folders on the way to `dir` that do not exist yet, shallowest first.
fn missing_ancestors(dir: &Path) -> Vec<PathBuf> {
    let mut missing: Vec<PathBuf> = dir
        .ancestors()
        .take_while(|ancestor| !ancestor.as_os_str().is_empty() && !ancestor.exists())
        .map(Path::to_path_buf)
        .collect();
    missing.reverse();
    missing
}

/// Remove folders that are empty, deepest first. `remove_dir` refuses a
/// folder that still holds anything, so this can never delete user files.
fn remove_empty_dirs(dirs: &[PathBuf]) {
    let mut ordered: Vec<&PathBuf> = dirs.iter().collect();
    ordered.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in ordered {
        let _ = fs::remove_dir(dir);
    }
}

pub fn undo_plan(mut plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    let roots = plan_roots(&plan);
    for index in 0..plan.actions.len() {
        let action = &mut plan.actions[index];
        if action.status != "executed" {
            continue;
        }
        if let Some(reason) = path_blocker(action, &roots) {
            action.status = "undo_failed".into();
            action.reason = reason.into();
            plan.errors
                .push(format!("{}: {}", action.destination_path, action.reason));
            continue;
        }
        let from = PathBuf::from(&action.destination_path);
        let to = PathBuf::from(&action.source_path);
        if !from.exists() {
            action.status = "undo_missing".into();
            action.reason = "되돌릴 대상이 없어 건너뜀 / destination no longer exists".into();
            plan.errors
                .push(format!("Undo missing: {}", from.display()));
            continue;
        }
        if to.exists() {
            action.status = "undo_conflict".into();
            action.reason =
                "원래 위치에 새 파일이 있어 덮어쓰지 않음 / original path has a newer file".into();
            plan.errors.push(format!("Undo conflict: {}", to.display()));
            continue;
        }
        if let Some(parent) = to.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                action.status = "undo_failed".into();
                action.reason =
                    format!("복원 폴더를 만들 수 없습니다 / cannot create restore folder: {error}");
                plan.errors
                    .push(format!("{}: {}", to.display(), action.reason));
                continue;
            }
        }
        match move_no_replace(&from, &to) {
            Ok(()) => action.status = "undone".into(),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                // A file appeared at the original path after the check above.
                action.status = "undo_conflict".into();
                action.reason =
                    "원래 위치에 새 파일이 있어 덮어쓰지 않음 / original path has a newer file"
                        .into();
                plan.errors.push(format!("Undo conflict: {}", to.display()));
            }
            Err(error) => {
                action.status = "undo_failed".into();
                action.reason = format!("복원 실패 / restore failed: {error}");
                plan.errors
                    .push(format!("{}: {}", from.display(), action.reason));
            }
        }
    }
    // Only after every file is back: a folder can be shared by several moves.
    let created: Vec<PathBuf> = plan
        .actions
        .iter()
        .filter(|action| action.status == "undone")
        .flat_map(|action| action.created_dirs.iter().map(PathBuf::from))
        .collect();
    remove_empty_dirs(&created);
    Ok(plan)
}

#[cfg(test)]
fn hash_file(path: &Path) -> io::Result<String> {
    hash_prefix(path, u64::MAX, &Ctl::none(), &|_| {})
}

/// SHA-256 of the first `limit` bytes (the whole file for `u64::MAX`).
/// `on_bytes` sees every chunk so progress can be bytes-accurate, and a
/// cancel is noticed between chunks.
fn hash_prefix(
    path: &Path,
    limit: u64,
    ctl: &Ctl,
    on_bytes: &(dyn Fn(u64) + Sync),
) -> io::Result<String> {
    // Last line of defence: opening a placeholder for reading downloads it.
    if is_cloud_only(&fs::metadata(path)?) {
        return Err(io::Error::other("cloud-only file is not read"));
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    // Worker threads have small stacks on Windows. Keep the streaming buffer
    // on the heap so a duplicate scan cannot overflow one.
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut remaining = limit;
    while remaining > 0 {
        if ctl.cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let want = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..want])?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        on_bytes(read as u64);
        remaining -= read as u64;
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn scan_entries(root: &str, sub: bool) -> impl Iterator<Item = Result<DirEntry, walkdir::Error>> {
    let walk = if sub {
        WalkDir::new(root)
    } else {
        WalkDir::new(root).max_depth(1)
    };
    walk.into_iter()
}

/// Files this large or smaller are fully covered by the quick hash.
const QUICK_BYTES: u64 = 64 * 1024;

/// Run `work` over `items` on a few threads, keeping the input order.
fn parallel_map<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    let threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(2)
        .clamp(1, 4)
        .min(items.len().max(1));
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, R)>> = Mutex::new(Vec::with_capacity(items.len()));
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                if index >= items.len() {
                    break;
                }
                let value = work(&items[index]);
                results
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push((index, value));
            });
        }
    });
    let mut ordered = results
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    ordered.sort_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, value)| value).collect()
}

#[cfg(test)]
pub fn find_duplicates(root: &str, sub: bool) -> Result<ScanResult<DuplicateGroup>, String> {
    find_duplicates_ctl(root, sub, &Ctl::none())
}

/// Size groups first, then a hash of the first 64 KB, then the full SHA-256
/// only for files that still look identical. Hashing runs on a few threads.
pub fn find_duplicates_ctl(
    root: &str,
    sub: bool,
    ctl: &Ctl,
) -> Result<ScanResult<DuplicateGroup>, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    if !Path::new(root).is_dir() {
        return Err(format!(
            "중복 파일을 찾을 폴더를 찾을 수 없습니다 / duplicate scan folder not found: {root}"
        ));
    }
    let mut sizes: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut cloud_only_skipped = 0;
    let mut seen: u64 = 0;
    for item in scan_entries(root, sub) {
        ctl.check()?;
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        seen += 1;
        ctl.report("scan", seen, 0, &entry.path().to_string_lossy(), false);
        if !entry.file_type().is_file() || protected(entry.path()) {
            continue;
        }
        if is_temporary_or_hidden(&entry, false, "safe") {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            if is_cloud_only(&metadata) {
                cloud_only_skipped += 1;
                continue;
            }
            sizes
                .entry(metadata.len())
                .or_default()
                .push(entry.into_path());
        }
    }

    // Stage 1: a hash of the first 64 KB separates almost everything that
    // merely shares a size, at a tiny fraction of the reading.
    let candidates: Vec<(u64, PathBuf)> = sizes
        .into_iter()
        .filter(|(_, files)| files.len() >= 2)
        .flat_map(|(size, files)| files.into_iter().map(move |path| (size, path)))
        .collect();
    let quick_done = AtomicU64::new(0);
    let quick_total = candidates.len() as u64;
    let quick = parallel_map(&candidates, |(size, path)| {
        let hash = hash_prefix(path, (*size).min(QUICK_BYTES), ctl, &|_| {}).ok();
        let done = quick_done.fetch_add(1, Ordering::Relaxed) + 1;
        ctl.report(
            "quick",
            done,
            quick_total,
            &path.to_string_lossy(),
            done == quick_total,
        );
        hash
    });
    ctl.check()?;
    let mut buckets: HashMap<(u64, String), Vec<PathBuf>> = HashMap::new();
    for ((size, path), hash) in candidates.into_iter().zip(quick) {
        if let Some(hash) = hash {
            buckets.entry((size, hash)).or_default().push(path);
        }
    }

    // Stage 2: the full SHA-256, only where the quick hashes agree. Files up
    // to 64 KB are already fully hashed.
    let mut groups = Vec::new();
    let mut need_full: Vec<(u64, PathBuf)> = Vec::new();
    for ((size, hash), files) in &buckets {
        if files.len() < 2 {
            continue;
        }
        if *size <= QUICK_BYTES {
            groups.push(DuplicateGroup {
                hash: hash.clone(),
                size_bytes: *size,
                reclaimable_bytes: size.saturating_mul(files.len() as u64 - 1),
                file_paths: files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            });
        } else {
            need_full.extend(files.iter().map(|path| (*size, path.clone())));
        }
    }
    let full_total: u64 = need_full.iter().map(|(size, _)| *size).sum();
    let full_done = AtomicU64::new(0);
    let full = parallel_map(&need_full, |(_, path)| {
        let name = path.to_string_lossy();
        hash_prefix(path, u64::MAX, ctl, &|bytes| {
            let done = full_done.fetch_add(bytes, Ordering::Relaxed) + bytes;
            ctl.report("hash", done, full_total, &name, false);
        })
        .ok()
    });
    ctl.check()?;
    ctl.report("hash", full_total, full_total, "", true);
    let mut by_hash: HashMap<(u64, String), Vec<String>> = HashMap::new();
    for ((size, path), hash) in need_full.into_iter().zip(full) {
        if let Some(hash) = hash {
            by_hash
                .entry((size, hash))
                .or_default()
                .push(path.display().to_string());
        }
    }
    for ((size, hash), paths) in by_hash {
        if paths.len() > 1 {
            groups.push(DuplicateGroup {
                hash,
                size_bytes: size,
                reclaimable_bytes: size.saturating_mul(paths.len() as u64 - 1),
                file_paths: paths,
            });
        }
    }
    // Stable order: biggest saving first, then by hash so runs are repeatable.
    groups.sort_by(|left, right| {
        right
            .reclaimable_bytes
            .cmp(&left.reclaimable_bytes)
            .then_with(|| left.hash.cmp(&right.hash))
    });
    for group in &mut groups {
        group.file_paths.sort();
    }
    Ok(ScanResult {
        items: groups,
        cloud_only_skipped,
    })
}

#[cfg(test)]
pub fn find_large_files(
    root: &str,
    sub: bool,
    minimum_size_bytes: u64,
) -> Result<ScanResult<LargeFile>, String> {
    find_large_files_ctl(root, sub, minimum_size_bytes, &Ctl::none())
}

pub fn find_large_files_ctl(
    root: &str,
    sub: bool,
    minimum_size_bytes: u64,
    ctl: &Ctl,
) -> Result<ScanResult<LargeFile>, String> {
    let mut files = Vec::new();
    let mut cloud_only_skipped = 0;
    let mut seen: u64 = 0;
    for item in scan_entries(root, sub) {
        ctl.check()?;
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        seen += 1;
        ctl.report("scan", seen, 0, &entry.path().to_string_lossy(), false);
        if !entry.file_type().is_file() || protected(entry.path()) {
            continue;
        }
        if is_temporary_or_hidden(&entry, false, "safe") {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            if is_cloud_only(&metadata) {
                cloud_only_skipped += 1;
                continue;
            }
            if metadata.len() >= minimum_size_bytes {
                files.push(LargeFile {
                    path: entry.path().display().to_string(),
                    size_bytes: metadata.len(),
                });
            }
        }
    }
    files.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    Ok(ScanResult {
        items: files,
        cloud_only_skipped,
    })
}

#[cfg(test)]
pub fn find_empty_folders(root: &str, sub: bool) -> Result<Vec<String>, String> {
    find_empty_folders_ctl(root, sub, &Ctl::none())
}

pub fn find_empty_folders_ctl(root: &str, sub: bool, ctl: &Ctl) -> Result<Vec<String>, String> {
    let mut folders = Vec::new();
    let mut seen: u64 = 0;
    for item in scan_entries(root, sub) {
        ctl.check()?;
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        seen += 1;
        ctl.report("scan", seen, 0, &entry.path().to_string_lossy(), false);
        if !entry.file_type().is_dir()
            || entry.path() == Path::new(root)
            || protected(entry.path())
            || project(entry.path())
            || is_temporary_or_hidden(&entry, false, "safe")
        {
            continue;
        }
        let has_children = match fs::read_dir(entry.path()) {
            Ok(mut children) => children.next().is_some(),
            Err(_) => continue,
        };
        // Marker files such as .gitkeep, desktop.ini and Thumbs.db still mean
        // the folder is not truly empty; never offer it as a deletion target.
        if !has_children {
            folders.push(entry.path().display().to_string());
        }
    }
    folders.sort();
    Ok(folders)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The system temp folder lives under %LOCALAPPDATA%, which the planner
    /// rightly refuses to organize. Keep test folders inside the build tree.
    fn tempdir() -> io::Result<tempfile::TempDir> {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        fs::create_dir_all(&base)?;
        tempfile::Builder::new().tempdir_in(base)
    }

    fn request(root: &Path) -> PlanRequest {
        PlanRequest {
            root_path: root.display().to_string(),
            include_subfolders: false,
            custom_categories: Vec::new(),
            language: "ko".into(),
            basis: "type".into(),
            mode: "safe".into(),
            date_basis: "modified".into(),
            include_hidden: false,
            excluded_paths: Vec::new(),
            excluded_extensions: Vec::new(),
            keyword_rules: Vec::new(),
        }
    }

    #[test]
    fn plans_hwp_and_collision_without_writes() {
        let directory = tempdir().expect("temp directory");
        fs::write(directory.path().join("문서.hwpx"), b"hwp").expect("source");
        fs::create_dir_all(directory.path().join("문서/한글")).expect("category");
        fs::write(directory.path().join("문서/한글/문서.hwpx"), b"existing").expect("collision");
        let plan = create_plan(request(directory.path())).expect("plan");
        assert_eq!(plan.actions.len(), 1);
        assert!(plan.actions[0].collision);
        assert!(plan.actions[0].destination_path.contains("(1)"));
        assert!(directory.path().join("문서.hwpx").exists());
    }

    #[test]
    fn custom_category_has_precedence() {
        let directory = tempdir().expect("temp directory");
        fs::write(directory.path().join("receipt.pdf"), b"pdf").expect("source");
        let mut req = request(directory.path());
        req.custom_categories.push(CustomCategory {
            name: "Receipts".into(),
            extensions: vec!["pdf".into()],
            enabled: true,
        });
        let plan = create_plan(req).expect("plan");
        assert_eq!(plan.actions[0].category, "Receipts");
    }

    #[test]
    fn date_day_basis_uses_year_month_day_folder() {
        let directory = tempdir().expect("temp directory");
        fs::write(directory.path().join("report.pdf"), b"pdf").expect("source");
        let mut req = request(directory.path());
        req.basis = "date_day".into();
        let plan = create_plan(req).expect("plan");
        let category = &plan.actions[0].category;
        let parts = category.split('-').collect::<Vec<_>>();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 4);
        assert!(parts[1].parse::<u32>().is_ok());
        assert!(parts[2].parse::<u32>().is_ok());
    }

    #[test]
    fn english_plan_uses_english_category_folders() {
        let directory = tempdir().expect("temp directory");
        fs::write(directory.path().join("photo.png"), b"png").expect("source");
        let mut req = request(directory.path());
        req.language = "en".into();
        let plan = create_plan(req).expect("plan");
        assert!(plan.actions[0].destination_path.contains("Images"));
    }

    #[test]
    fn duplicate_scan_uses_full_sha256() {
        let directory = tempdir().expect("temp directory");
        fs::write(directory.path().join("a.txt"), b"same").expect("a");
        fs::write(directory.path().join("b.txt"), b"same").expect("b");
        let groups = find_duplicates(&directory.path().display().to_string(), false)
            .expect("scan")
            .items;
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].file_paths.len(), 2);
        assert_eq!(groups[0].hash.len(), 64);
    }

    #[test]
    fn duplicate_hashing_keeps_worker_stack_usage_bounded() {
        let directory = tempdir().expect("temp directory");
        let path = directory.path().join("large.bin");
        fs::write(&path, vec![7_u8; 2 * 1024 * 1024]).expect("large file");
        let handle = std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(move || hash_file(&path))
            .expect("worker thread");
        let hash = handle
            .join()
            .expect("worker should not overflow its stack")
            .expect("hash");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn preview_reserves_duplicate_basenames_across_subfolders() {
        let directory = tempdir().expect("temp directory");
        fs::create_dir_all(directory.path().join("one")).expect("one");
        fs::create_dir_all(directory.path().join("two")).expect("two");
        fs::write(directory.path().join("one/report.pdf"), b"one").expect("one report");
        fs::write(directory.path().join("two/report.pdf"), b"two").expect("two report");
        let mut req = request(directory.path());
        req.include_subfolders = true;
        let plan = create_plan(req).expect("plan");
        assert_eq!(plan.actions.len(), 2);
        assert_ne!(
            plan.actions[0].destination_path,
            plan.actions[1].destination_path
        );
        assert!(plan.actions.iter().any(|action| action.collision));
    }

    #[test]
    fn marker_only_folder_is_not_reported_as_empty() {
        let directory = tempdir().expect("temp directory");
        let marker_folder = directory.path().join("keep");
        fs::create_dir_all(&marker_folder).expect("folder");
        fs::write(marker_folder.join(".gitkeep"), b"").expect("marker");
        let folders =
            find_empty_folders(&directory.path().display().to_string(), true).expect("scan");
        assert!(!folders.contains(&marker_folder.display().to_string()));
    }

    #[test]
    fn undo_never_overwrites_original_file() {
        let directory = tempdir().expect("temp directory");
        let original = directory.path().join("a.txt");
        let moved = directory.path().join("문서").join("a.txt");
        fs::create_dir_all(moved.parent().unwrap()).expect("category");
        fs::write(&moved, b"moved").expect("moved");
        fs::write(&original, b"newer").expect("newer");
        let plan = OrganizationPlan {
            root_path: directory.path().display().to_string(),
            actions: vec![PlannedAction {
                source_path: original.display().to_string(),
                destination_path: moved.display().to_string(),
                category: "문서".into(),
                reason: "test".into(),
                status: "executed".into(),
                size_bytes: 5,
                modified_unix_secs: None,
                collision: false,
                created_dirs: Vec::new(),
            }],
            exclusions: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            basis: "type".into(),
            language: "ko".into(),
            mode: "safe".into(),
            root_paths: vec![directory.path().display().to_string()],
            run_id: String::new(),
            plan_id: String::new(),
        };
        let result = undo_plan(plan).expect("undo");
        assert_eq!(result.actions[0].status, "undo_conflict");
        assert_eq!(fs::read(&original).unwrap(), b"newer");
        assert!(moved.exists());
    }

    #[test]
    fn move_no_replace_never_overwrites_an_existing_file() {
        let directory = tempdir().expect("temp directory");
        let from = directory.path().join("from.txt");
        let to = directory.path().join("to.txt");
        fs::write(&from, b"incoming").expect("from");
        fs::write(&to, b"existing").expect("to");
        let error = move_no_replace(&from, &to).expect_err("must refuse");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&from).unwrap(), b"incoming");
        assert_eq!(fs::read(&to).unwrap(), b"existing");

        let free = directory.path().join("free.txt");
        move_no_replace(&from, &free).expect("move to a free name");
        assert!(!from.exists());
        assert_eq!(fs::read(&free).unwrap(), b"incoming");
    }

    #[test]
    fn move_with_unique_name_numbers_a_taken_destination() {
        let directory = tempdir().expect("temp directory");
        let from = directory.path().join("from.txt");
        let taken = directory.path().join("a.txt");
        fs::write(&from, b"incoming").expect("from");
        fs::write(&taken, b"existing").expect("taken");
        let (moved, renamed) = move_with_unique_name(&from, taken.clone()).expect("move");
        assert!(renamed);
        assert_eq!(moved, directory.path().join("a (1).txt"));
        assert_eq!(fs::read(&taken).unwrap(), b"existing");
        assert_eq!(fs::read(&moved).unwrap(), b"incoming");
    }

    #[test]
    fn execute_keeps_a_file_that_appeared_after_planning() {
        let directory = tempdir().expect("temp directory");
        let source = directory.path().join("a.txt");
        let destination = directory.path().join("문서").join("a.txt");
        fs::write(&source, b"incoming").expect("source");
        let plan = OrganizationPlan {
            root_path: directory.path().display().to_string(),
            actions: vec![PlannedAction {
                source_path: source.display().to_string(),
                destination_path: destination.display().to_string(),
                category: "문서".into(),
                reason: "test".into(),
                status: "proposed".into(),
                size_bytes: 8,
                modified_unix_secs: None,
                collision: false,
                created_dirs: Vec::new(),
            }],
            exclusions: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            basis: "type".into(),
            language: "ko".into(),
            mode: "safe".into(),
            root_paths: vec![directory.path().display().to_string()],
            run_id: String::new(),
            plan_id: String::new(),
        };
        // The file shows up after the preview was built, before execution.
        fs::create_dir_all(destination.parent().unwrap()).expect("category");
        fs::write(&destination, b"arrived later").expect("late file");
        let done = execute_plan_with(plan, |_, _| Ok(())).expect("execute");
        assert_eq!(done.actions[0].status, "executed");
        assert!(done.actions[0].collision);
        assert_eq!(fs::read(&destination).unwrap(), b"arrived later");
        let renamed = PathBuf::from(&done.actions[0].destination_path);
        assert_ne!(renamed, destination);
        assert_eq!(fs::read(&renamed).unwrap(), b"incoming");
    }

    #[test]
    fn cloud_placeholder_attributes_are_recognised() {
        assert!(attributes_mean_cloud_only(0x40_0000)); // RECALL_ON_DATA_ACCESS
        assert!(attributes_mean_cloud_only(0x4_0000)); // RECALL_ON_OPEN
        assert!(attributes_mean_cloud_only(0x1000 | 0x20)); // OFFLINE + ARCHIVE
        assert!(!attributes_mean_cloud_only(0x20)); // ARCHIVE only
        assert!(!attributes_mean_cloud_only(0));
    }

    #[cfg(windows)]
    fn mark_offline(path: &Path) {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::SetFileAttributesW;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        // FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_ARCHIVE
        assert_ne!(
            unsafe { SetFileAttributesW(wide.as_ptr(), 0x1000 | 0x20) },
            0
        );
    }

    #[cfg(windows)]
    #[test]
    fn scans_leave_cloud_only_files_alone_and_count_them() {
        let directory = tempdir().expect("temp directory");
        let local_a = directory.path().join("local-a.bin");
        let local_b = directory.path().join("local-b.bin");
        let cloud_a = directory.path().join("cloud-a.bin");
        let cloud_b = directory.path().join("cloud-b.bin");
        for path in [&local_a, &local_b, &cloud_a, &cloud_b] {
            fs::write(path, vec![9_u8; 4096]).expect("file");
        }
        mark_offline(&cloud_a);
        mark_offline(&cloud_b);
        let root = directory.path().display().to_string();

        let duplicates = find_duplicates(&root, false).expect("duplicates");
        assert_eq!(duplicates.cloud_only_skipped, 2);
        assert_eq!(duplicates.items.len(), 1);
        let paths = &duplicates.items[0].file_paths;
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|path| path.contains("local-")));

        let large = find_large_files(&root, false, 1024).expect("large");
        assert_eq!(large.cloud_only_skipped, 2);
        assert_eq!(large.items.len(), 2);
        assert!(hash_file(&cloud_a).is_err());
        assert!(hash_file(&local_a).is_ok());
    }

    fn plan_with(root: &Path, pairs: &[(&Path, &Path)]) -> OrganizationPlan {
        OrganizationPlan {
            root_path: root.display().to_string(),
            actions: pairs
                .iter()
                .map(|(source, destination)| PlannedAction {
                    source_path: source.display().to_string(),
                    destination_path: destination.display().to_string(),
                    category: "Test".into(),
                    reason: "test".into(),
                    status: "proposed".into(),
                    size_bytes: 1,
                    modified_unix_secs: None,
                    collision: false,
                    created_dirs: Vec::new(),
                })
                .collect(),
            exclusions: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            basis: "type".into(),
            language: "ko".into(),
            mode: "safe".into(),
            root_paths: vec![root.display().to_string()],
            run_id: String::new(),
            plan_id: String::new(),
        }
    }

    #[test]
    fn undo_removes_only_the_folders_the_run_created() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        let (a, b, c) = (root.join("a.txt"), root.join("b.txt"), root.join("c.txt"));
        for file in [&a, &b, &c] {
            fs::write(file, b"x").expect("source");
        }
        // `Kept` existed before the run (and is empty); `Docs/Text` is new and shared.
        fs::create_dir_all(root.join("Kept")).expect("pre-existing folder");
        let plan = plan_with(
            root,
            &[
                (&a, &root.join("Docs").join("Text").join("a.txt")),
                (&b, &root.join("Docs").join("Text").join("b.txt")),
                (&c, &root.join("Kept").join("c.txt")),
            ],
        );
        let done = execute_plan_with(plan, |_, _| Ok(())).expect("execute");
        assert_eq!(done.actions[0].created_dirs.len(), 2); // Docs, Docs/Text
        assert!(done.actions[1].created_dirs.is_empty()); // folder already made by the first move
        assert!(done.actions[2].created_dirs.is_empty()); // Kept pre-existed

        let undone = undo_plan(done).expect("undo");
        assert!(undone
            .actions
            .iter()
            .all(|action| action.status == "undone"));
        assert!(a.is_file() && b.is_file() && c.is_file());
        assert!(!root.join("Docs").exists(), "new folders are cleaned up");
        assert!(root.join("Kept").is_dir(), "a folder the user had stays");
    }

    #[test]
    fn undo_keeps_a_created_folder_that_now_holds_other_files() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        let a = root.join("a.txt");
        fs::write(&a, b"x").expect("source");
        let plan = plan_with(root, &[(&a, &root.join("Docs").join("a.txt"))]);
        let done = execute_plan_with(plan, |_, _| Ok(())).expect("execute");
        fs::write(root.join("Docs").join("mine.txt"), b"keep me").expect("user file");
        undo_plan(done).expect("undo");
        assert!(a.is_file());
        assert_eq!(
            fs::read(root.join("Docs").join("mine.txt")).unwrap(),
            b"keep me"
        );
    }

    #[test]
    fn a_failed_move_does_not_leave_the_folders_it_created() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        let missing = root.join("not-there.txt");
        // The source vanished before execution: nothing may be created.
        let plan = plan_with(root, &[(&missing, &root.join("Docs").join("x.txt"))]);
        let done = execute_plan_with(plan, |_, _| Ok(())).expect("execute");
        assert_eq!(done.actions[0].status, "skipped");
        assert!(!root.join("Docs").exists());
    }

    #[test]
    fn drive_roots_profile_and_system_folders_cannot_be_organised() {
        assert!(root_rejection(Path::new("C:\\")).is_some());
        assert!(root_rejection(Path::new("D:\\")).is_some());
        assert!(root_rejection(Path::new(r"C:\$Recycle.Bin")).is_some());
        assert!(root_rejection(Path::new(r"E:\System Volume Information")).is_some());
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            let profile = PathBuf::from(profile);
            assert!(
                root_rejection(&profile).is_some(),
                "the profile root itself"
            );
            if let Some(parent) = profile.parent() {
                assert!(root_rejection(parent).is_some(), "a parent of the profile");
            }
            // A folder inside the profile is fine (it is not under AppData).
            assert!(root_rejection(&profile.join("Downloads")).is_none());
        }
    }

    #[test]
    fn data_drive_folders_named_like_system_folders_are_allowed() {
        // `Recovery` is only special on the Windows drive.
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let other = if system.to_ascii_lowercase().starts_with('d') {
            "E:"
        } else {
            "D:"
        };
        assert!(root_rejection(Path::new(&format!(r"{other}\Recovery\mine"))).is_none());
        assert!(root_rejection(Path::new(&format!(r"{system}\Recovery"))).is_some());
    }

    #[cfg(windows)]
    #[test]
    fn short_8_3_names_do_not_bypass_protection() {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
        // "Program Files" has a distinct 8.3 alias (PROGRA~1) when the volume keeps them.
        let long = PathBuf::from(std::env::var_os("ProgramFiles").expect("ProgramFiles"));
        let wide: Vec<u16> = long.as_os_str().encode_wide().chain([0]).collect();
        let mut buffer = vec![0_u16; 1024];
        let length =
            unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
        assert!(length > 0, "GetShortPathNameW failed");
        let short = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]));
        if path_key(&short) == path_key(&long) {
            eprintln!("8.3 names are disabled on this volume; bypass cannot be exercised");
            return;
        }
        // The spelling alone no longer matches, so only resolution can catch it.
        assert!(!path_is_under(&short, &long));
        assert!(
            is_protected_path(&short),
            "{short:?} must resolve to {long:?}"
        );
        assert!(root_rejection(&short).is_some());
    }

    fn touch(dir: &Path, name: &str) {
        fs::create_dir_all(dir).expect("dir");
        fs::write(dir.join(name), b"x").expect("file");
    }

    #[test]
    fn one_stray_manifest_is_not_a_project() {
        let directory = tempdir().expect("temp directory");
        for lone in ["package.json", "requirements.txt", "Makefile", "app.sln"] {
            let dir = directory.path().join(lone.replace('.', "_"));
            touch(&dir, lone);
            assert!(!project(&dir), "{lone} alone must not protect the folder");
        }
    }

    #[test]
    fn repositories_and_real_projects_are_detected() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        fs::create_dir_all(root.join("repo").join(".git")).expect("git");
        assert!(project(&root.join("repo")), "a .git folder");
        fs::create_dir_all(root.join("svn").join(".svn")).expect("svn");
        assert!(project(&root.join("svn")), "a .svn folder");

        touch(&root.join("two"), "package.json");
        touch(&root.join("two"), "requirements.txt");
        assert!(project(&root.join("two")), "two manifests");

        touch(&root.join("withsrc"), "go.mod");
        fs::create_dir_all(root.join("withsrc").join("src")).expect("src");
        assert!(project(&root.join("withsrc")), "a manifest plus src");

        touch(&root.join("vs"), "app.sln");
        touch(&root.join("vs"), "app.csproj");
        assert!(project(&root.join("vs")), "solution plus project file");
    }

    #[test]
    fn a_stray_package_json_no_longer_blocks_organising_the_folder() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        fs::write(root.join("package.json"), b"{}").expect("manifest");
        fs::write(root.join("photo.png"), b"png").expect("photo");
        // A real project below the root is still left alone.
        touch(&root.join("app"), "package.json");
        touch(&root.join("app"), "requirements.txt");
        fs::write(root.join("app").join("notes.txt"), b"keep").expect("inside project");

        let mut req = request(root);
        req.include_subfolders = true;
        let plan = create_plan(req).expect("plan");
        let moved: Vec<String> = plan
            .actions
            .iter()
            .map(|action| action.source_path.clone())
            .collect();
        assert!(moved.iter().any(|path| path.ends_with("photo.png")));
        assert!(moved.iter().any(|path| path.ends_with("package.json")));
        assert!(
            !moved.iter().any(|path| path.contains("app")),
            "files inside the real project stay put: {moved:?}"
        );
        assert!(plan.exclusions.iter().any(|note| note.contains("app")));
    }

    #[test]
    fn execution_refuses_paths_outside_the_chosen_folder() {
        let directory = tempdir().expect("temp directory");
        let outside = tempdir().expect("outside");
        let root = directory.path();
        let inside_file = root.join("a.txt");
        let outside_file = outside.path().join("secret.txt");
        fs::write(&inside_file, b"a").expect("inside");
        fs::write(&outside_file, b"secret").expect("outside");
        let escape = root.join("..").join("escaped.txt");
        let plan = plan_with(
            root,
            &[
                (&outside_file, &root.join("Docs").join("secret.txt")), // source outside the folder
                (&inside_file, &outside.path().join("a.txt")),          // destination outside
                (&inside_file, &escape),                                // `..` traversal
            ],
        );
        let done = execute_plan_with(plan, |_, _| Ok(())).expect("execute");
        assert!(
            done.actions.iter().all(|action| action.status == "skipped"),
            "{:?}",
            done.actions.iter().map(|a| &a.reason).collect::<Vec<_>>()
        );
        assert!(outside_file.is_file() && inside_file.is_file());
        assert!(!outside.path().join("a.txt").exists());
        assert!(!root.join("..").join("escaped.txt").exists());
    }

    #[test]
    fn undo_refuses_a_record_that_points_outside_the_folder() {
        let directory = tempdir().expect("temp directory");
        let outside = tempdir().expect("outside");
        let root = directory.path();
        let moved = root.join("Docs").join("a.txt");
        fs::create_dir_all(moved.parent().unwrap()).expect("folder");
        fs::write(&moved, b"a").expect("moved");
        let planted = outside.path().join("restored.txt");
        let mut plan = plan_with(root, &[(&planted, &moved)]);
        plan.actions[0].status = "executed".into();
        let undone = undo_plan(plan).expect("undo");
        assert_eq!(undone.actions[0].status, "undo_failed");
        assert!(moved.is_file(), "the file was not touched");
        assert!(!planted.exists());
    }

    #[test]
    fn files_changed_after_the_preview_are_skipped() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        let (kept, edited) = (root.join("kept.txt"), root.join("edited.txt"));
        fs::write(&kept, b"same").expect("kept");
        fs::write(&edited, b"before").expect("edited");
        let mut plan = create_plan(request(root)).expect("plan");
        assert_eq!(plan.actions.len(), 2);
        fs::write(&edited, b"edited after the preview, now longer").expect("edit");
        mark_changed_since_preview(&mut plan);
        let status = |name: &str| {
            plan.actions
                .iter()
                .find(|action| action.source_path.ends_with(name))
                .map(|action| action.status.clone())
                .unwrap()
        };
        assert_eq!(status("kept.txt"), "proposed");
        assert_eq!(status("edited.txt"), "skipped");
    }

    #[test]
    fn same_size_and_same_first_64kb_but_different_tail_are_not_duplicates() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        let size = 300 * 1024;
        let mut a = vec![1_u8; size];
        let mut b = a.clone();
        let c = a.clone();
        a[size - 1] = 2; // differs only in the last byte, far past the quick hash
        b[size - 1] = 3;
        fs::write(root.join("a.bin"), &a).expect("a");
        fs::write(root.join("b.bin"), &b).expect("b");
        fs::write(root.join("c1.bin"), &c).expect("c1");
        fs::write(root.join("c2.bin"), &c).expect("c2");
        // Small files are fully covered by the quick hash.
        fs::write(root.join("s1.txt"), b"tiny").expect("s1");
        fs::write(root.join("s2.txt"), b"tiny").expect("s2");

        let found = find_duplicates(&root.display().to_string(), false)
            .expect("scan")
            .items;
        assert_eq!(found.len(), 2, "{found:?}");
        let big = found
            .iter()
            .find(|group| group.size_bytes == size as u64)
            .expect("big group");
        assert_eq!(big.file_paths.len(), 2);
        assert!(big
            .file_paths
            .iter()
            .all(|path| path.contains("c1") || path.contains("c2")));
        assert_eq!(big.hash.len(), 64, "a full SHA-256, not the quick one");
        let small = found
            .iter()
            .find(|group| group.size_bytes == 4)
            .expect("small group");
        assert_eq!(small.file_paths.len(), 2);
        assert_eq!(small.hash.len(), 64);
    }

    #[test]
    fn duplicate_results_are_stable_across_runs() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        for index in 0..12 {
            fs::write(
                root.join(format!("x{index}.bin")),
                vec![(index % 3) as u8; 100 * 1024],
            )
            .expect("file");
        }
        let run = || {
            find_duplicates(&root.display().to_string(), false)
                .expect("scan")
                .items
                .iter()
                .map(|group| group.file_paths.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "threads must not change the order of results");
    }

    #[test]
    fn a_cancelled_scan_or_plan_returns_the_cancel_error() {
        use crate::progress::{Operations, CANCELLED};
        let directory = tempdir().expect("temp directory");
        fs::write(directory.path().join("a.bin"), b"x").expect("a");
        fs::write(directory.path().join("b.bin"), b"x").expect("b");
        let root = directory.path().display().to_string();

        let operations = Operations::default();
        let ctl = operations.begin(None).expect("begin");
        operations.cancel();
        assert_eq!(
            find_duplicates_ctl(&root, false, &ctl).unwrap_err(),
            CANCELLED
        );
        assert_eq!(
            find_large_files_ctl(&root, false, 0, &ctl).unwrap_err(),
            CANCELLED
        );
        assert_eq!(
            find_empty_folders_ctl(&root, false, &ctl).unwrap_err(),
            CANCELLED
        );
        assert_eq!(
            create_plan_ctl(request(directory.path()), &ctl).unwrap_err(),
            CANCELLED
        );
    }

    #[test]
    fn cancelling_during_a_move_stops_after_the_current_file() {
        use crate::progress::Operations;
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        let names = ["a.txt", "b.txt", "c.txt", "d.txt"];
        for name in names {
            fs::write(root.join(name), b"x").expect("source");
        }
        let destinations: Vec<PathBuf> = names.iter().map(|n| root.join("Docs").join(n)).collect();
        let sources: Vec<PathBuf> = names.iter().map(|n| root.join(n)).collect();
        let pairs: Vec<(&Path, &Path)> = sources
            .iter()
            .zip(&destinations)
            .map(|(s, d)| (s.as_path(), d.as_path()))
            .collect();
        let plan = plan_with(root, &pairs);

        let operations = Operations::default();
        let ctl = operations.begin(None).expect("begin");
        // The user presses Cancel while the second file is being settled.
        let done = execute_plan_ctl(plan, &ctl, |index, _| {
            if index == 1 {
                operations.cancel();
            }
            Ok(())
        })
        .expect("execute");
        let statuses: Vec<&str> = done.actions.iter().map(|a| a.status.as_str()).collect();
        assert_eq!(statuses, ["executed", "executed", "skipped", "skipped"]);
        assert!(destinations[1].is_file() && sources[2].is_file() && sources[3].is_file());
        assert!(done.actions[3].reason.contains("cancel"));
        // What did move is still undoable.
        let undone = undo_plan(done).expect("undo");
        assert!(sources.iter().all(|source| source.is_file()));
        assert!(!root.join("Docs").exists());
        assert_eq!(undone.actions[0].status, "undone");
    }

    #[test]
    fn scans_and_moves_report_progress() {
        use crate::progress::{Operations, Progress};
        use std::sync::{Arc, Mutex};
        let directory = tempdir().expect("temp directory");
        let root = directory.path();
        for name in ["a.bin", "b.bin"] {
            fs::write(root.join(name), vec![5_u8; 200 * 1024]).expect("file");
        }
        let events: Arc<Mutex<Vec<Progress>>> = Arc::default();
        let sink = events.clone();
        let operations = Operations::default();
        let ctl = operations
            .begin(Some(Box::new(move |progress| {
                sink.lock().unwrap().push(progress)
            })))
            .expect("begin");
        let found = find_duplicates_ctl(&root.display().to_string(), false, &ctl).expect("scan");
        assert_eq!(found.items.len(), 1);
        let seen = events.lock().unwrap().clone();
        let phases: Vec<&str> = seen.iter().map(|event| event.phase).collect();
        assert!(
            phases.contains(&"scan") || phases.contains(&"quick"),
            "{phases:?}"
        );
        let last_hash = seen
            .iter()
            .rev()
            .find(|event| event.phase == "hash")
            .expect("a hash event");
        assert_eq!(
            last_hash.done, last_hash.total,
            "the final update reports completion"
        );
        assert_eq!(
            last_hash.total,
            2 * 200 * 1024,
            "bytes to hash = the two candidate files"
        );
    }
}
