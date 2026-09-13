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

const PROJECT_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    "requirements.txt",
    "pyproject.toml",
    "cargo.toml",
    "pom.xml",
    "build.gradle",
    "composer.json",
    "go.mod",
    "cmakelists.txt",
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

fn path_key(path: &Path) -> String {
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

fn protected(path: &Path) -> bool {
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
    let app_data = std::env::var_os("APPDATA")
        .into_iter()
        .chain(std::env::var_os("LOCALAPPDATA"))
        .map(|value| path_is_under(path, Path::new(&value)))
        .any(|value| value);
    system_root
        || program_root
        || app_data
        || component_match(".git")
        || component_match("node_modules")
}

/// Shared safety check for commands that operate on paths selected in the UI.
pub fn is_protected_path(path: &Path) -> bool {
    protected(path)
}

fn project(path: &Path) -> bool {
    PROJECT_MARKERS
        .iter()
        .any(|marker| path.join(marker).exists())
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

pub fn create_plan(req: PlanRequest) -> Result<OrganizationPlan, String> {
    let root = PathBuf::from(&req.root_path);
    if !root.is_dir() {
        return Err("선택한 폴더를 찾을 수 없습니다. / The selected folder does not exist.".into());
    }
    if protected(&root) || project(&root) {
        return Err("보호된 시스템 또는 개발 프로젝트 경로는 정리할 수 없습니다. / Protected system or development project path.".into());
    }
    let category_roots = planned_category_roots(&root, &req);
    let mut actions = Vec::new();
    let mut exclusions = Vec::new();
    let mut errors = Vec::new();
    let mut reserved_destinations = HashSet::new();
    let warnings = root_warnings(&root);
    let walker = if req.include_subfolders {
        WalkDir::new(&root).min_depth(1)
    } else {
        WalkDir::new(&root).min_depth(1).max_depth(1)
    };
    for item in walker {
        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(format!("스캔 오류 / Scan error: {error}"));
                continue;
            }
        };
        let path = entry.path();
        if entry.file_type().is_dir() {
            if path != root && project(path) {
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
        if path
            .ancestors()
            .skip(1)
            .any(|parent| parent != root && project(parent))
        {
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
    })
}

/// Build one dry-run snapshot for several independently selected roots. Each
/// root is scanned with the same settings, while moves never cross root
/// boundaries.
pub fn create_multi_plan(requests: Vec<PlanRequest>) -> Result<OrganizationPlan, String> {
    if requests.is_empty() {
        return Err("정리할 폴더를 하나 이상 선택하세요. / Select at least one folder.".into());
    }
    let mut plans = Vec::with_capacity(requests.len());
    for request in requests {
        plans.push(create_plan(request)?);
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
    })
}

pub fn execute_plan(mut plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    for index in 0..plan.actions.len() {
        let action = &mut plan.actions[index];
        if action.status != "proposed" {
            continue;
        }
        let from_path = PathBuf::from(&action.source_path);
        if !from_path.is_file() {
            action.status = "skipped".into();
            action.reason = "원본 파일이 없어 건너뜀 / source no longer exists".into();
            continue;
        }
        let mut destination = PathBuf::from(&action.destination_path);
        if path_key(&from_path) == path_key(&destination) {
            action.status = "skipped".into();
            continue;
        }
        if destination.exists() {
            let (safe, collision) = safe_destination(destination.clone());
            destination = safe;
            action.collision = action.collision || collision;
            action.destination_path = destination.display().to_string();
        }
        if let Some(parent) = destination.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                action.status = "failed".into();
                action.reason =
                    format!("폴더를 만들 수 없습니다 / cannot create destination folder: {error}");
                plan.errors
                    .push(format!("{}: {}", from_path.display(), action.reason));
                continue;
            }
        }
        match fs::rename(&from_path, &destination) {
            Ok(_) => {
                action.destination_path = destination.display().to_string();
                action.status = "executed".into();
            }
            Err(error) => {
                action.status = "failed".into();
                action.reason = format!("이동 실패 / move failed: {error}");
                plan.errors
                    .push(format!("{}: {}", from_path.display(), action.reason));
            }
        }
    }
    Ok(plan)
}

pub fn undo_plan(mut plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    for index in 0..plan.actions.len() {
        let action = &mut plan.actions[index];
        if action.status != "executed" {
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
        match fs::rename(&from, &to) {
            Ok(_) => action.status = "undone".into(),
            Err(error) => {
                action.status = "undo_failed".into();
                action.reason = format!("복원 실패 / restore failed: {error}");
                plan.errors
                    .push(format!("{}: {}", from.display(), action.reason));
            }
        }
    }
    Ok(plan)
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    // Tauri IPC workers use relatively small stacks on Windows. Keep the
    // streaming buffer on the heap so a duplicate scan cannot overflow one.
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
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

pub fn find_duplicates(root: &str, sub: bool) -> Result<Vec<DuplicateGroup>, String> {
    if !Path::new(root).is_dir() {
        return Err(format!(
            "중복 파일을 찾을 폴더를 찾을 수 없습니다 / duplicate scan folder not found: {root}"
        ));
    }
    let mut sizes: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for item in scan_entries(root, sub) {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() || protected(entry.path()) {
            continue;
        }
        if is_temporary_or_hidden(&entry, false, "safe") {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            sizes
                .entry(metadata.len())
                .or_default()
                .push(entry.into_path());
        }
    }
    let mut groups = Vec::new();
    for (size, files) in sizes {
        if files.len() < 2 {
            continue;
        }
        let mut hashes: HashMap<String, Vec<String>> = HashMap::new();
        for path in files {
            if let Ok(hash) = hash_file(&path) {
                hashes
                    .entry(hash)
                    .or_default()
                    .push(path.display().to_string());
            }
        }
        for (hash, paths) in hashes {
            if paths.len() > 1 {
                groups.push(DuplicateGroup {
                    hash,
                    size_bytes: size,
                    reclaimable_bytes: size.saturating_mul(paths.len() as u64 - 1),
                    file_paths: paths,
                });
            }
        }
    }
    groups.sort_by(|left, right| right.reclaimable_bytes.cmp(&left.reclaimable_bytes));
    Ok(groups)
}

pub fn find_large_files(
    root: &str,
    sub: bool,
    minimum_size_bytes: u64,
) -> Result<Vec<LargeFile>, String> {
    let mut files = Vec::new();
    for item in scan_entries(root, sub) {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() || protected(entry.path()) {
            continue;
        }
        if is_temporary_or_hidden(&entry, false, "safe") {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            if metadata.len() >= minimum_size_bytes {
                files.push(LargeFile {
                    path: entry.path().display().to_string(),
                    size_bytes: metadata.len(),
                });
            }
        }
    }
    files.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    Ok(files)
}

pub fn find_empty_folders(root: &str, sub: bool) -> Result<Vec<String>, String> {
    let mut folders = Vec::new();
    for item in scan_entries(root, sub) {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
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
    use tempfile::tempdir;

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
        let groups = find_duplicates(&directory.path().display().to_string(), false).expect("scan");
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
            }],
            exclusions: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            created_at: Utc::now().to_rfc3339(),
            basis: "type".into(),
            language: "ko".into(),
            mode: "safe".into(),
            root_paths: vec![directory.path().display().to_string()],
        };
        let result = undo_plan(plan).expect("undo");
        assert_eq!(result.actions[0].status, "undo_conflict");
        assert_eq!(fs::read(&original).unwrap(), b"newer");
        assert!(moved.exists());
    }
}
