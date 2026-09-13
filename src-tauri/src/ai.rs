//! AI provider configuration and connection checks.
//!
//! API keys never leave this module in a response.  On Windows they are
//! protected with DPAPI and stored alongside non-secret provider preferences
//! in the per-user application-data directory.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Once,
    time::Duration,
};
use tauri::{AppHandle, Manager};

const OLLAMA_ENDPOINT: &str = "http://127.0.0.1:11434";
const CONFIG_FILE: &str = "ai-providers.json";
static TLS_PROVIDER: Once = Once::new();

fn http_client(timeout: Duration) -> Result<Client, String> {
    TLS_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("네트워크 클라이언트를 만들 수 없습니다: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSetting {
    pub id: String,
    pub display_name: String,
    pub model: String,
    pub key_present: bool,
    pub enabled: bool,
    pub registration_url: String,
    pub endpoint: String,
    pub ollama_models: Vec<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub detected: bool,
    pub endpoint: String,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiFileMetadata {
    pub id: usize,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_unix_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestion {
    pub id: usize,
    pub category: String,
    pub confidence: f32,
    pub reason: String,
    pub is_new_category: bool,
    pub needs_review: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiClassificationResult {
    pub provider_id: String,
    pub model: String,
    pub request_count: usize,
    pub suggestions: Vec<AiSuggestion>,
    pub proposed_categories: Vec<String>,
}

#[tauri::command]
pub fn apply_ai_suggestions(
    mut plan: crate::planner::OrganizationPlan,
    suggestions: Vec<AiSuggestion>,
) -> Result<crate::planner::OrganizationPlan, String> {
    let language = plan.language.clone();
    let mut reserved = HashSet::new();
    for suggestion in suggestions {
        let Some(existing) = plan.actions.get(suggestion.id) else {
            continue;
        };
        if existing.status != "proposed" {
            continue;
        }
        let source_path = existing.source_path.clone();
        let root = crate::planner::root_for_source(&plan, Path::new(&source_path));
        let Some(action) = plan.actions.get_mut(suggestion.id) else {
            continue;
        };
        let category = crate::planner::sanitize_category(&suggestion.category, &language);
        let file_name = Path::new(&action.source_path)
            .file_name()
            .ok_or_else(|| "AI 제안의 원본 파일명을 확인할 수 없습니다.".to_string())?;
        let base = root.join(&category).join(file_name);
        let mut destination = base.clone();
        let mut collision = false;
        if destination.exists() || reserved.contains(&destination) {
            collision = true;
            let stem = base
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("file");
            let extension = base
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            for number in 1..10_000 {
                let name = if extension.is_empty() {
                    format!("{stem} ({number})")
                } else {
                    format!("{stem} ({number}).{extension}")
                };
                let candidate = base.with_file_name(name);
                if !candidate.exists() && !reserved.contains(&candidate) {
                    destination = candidate;
                    break;
                }
            }
        }
        reserved.insert(destination.clone());
        action.category = category;
        action.destination_path = destination.display().to_string();
        action.reason = format!(
            "AI: {}",
            suggestion.reason.chars().take(240).collect::<String>()
        );
        action.status = "proposed".into();
        action.collision = action.collision || collision;
    }
    Ok(plan)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredConfig {
    providers: BTreeMap<String, StoredProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredProvider {
    model: String,
    #[serde(default)]
    encrypted_key: Vec<u8>,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn provider_defaults(id: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match id {
        "openai" => (
            "OpenAI",
            "gpt-4o-mini",
            "https://api.openai.com",
            "https://platform.openai.com/api-keys",
        ),
        "claude" => (
            "Anthropic Claude",
            "claude-3-5-haiku-latest",
            "https://api.anthropic.com",
            "https://console.anthropic.com/settings/keys",
        ),
        "gemini" => (
            "Google Gemini",
            "gemini-2.0-flash",
            "https://generativelanguage.googleapis.com",
            "https://aistudio.google.com/app/apikey",
        ),
        "ollama" => (
            "Ollama (로컬)",
            "",
            OLLAMA_ENDPOINT,
            "https://ollama.com/download/windows",
        ),
        _ => ("알 수 없는 공급자", "", "", ""),
    }
}

fn provider_ids() -> [&'static str; 4] {
    ["openai", "claude", "gemini", "ollama"]
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("앱 데이터 경로를 확인할 수 없습니다: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("앱 데이터 폴더를 만들 수 없습니다: {e}"))?;
    Ok(dir.join(CONFIG_FILE))
}

fn read_config(app: &AppHandle) -> Result<StoredConfig, String> {
    let path = config_path(app)?;
    match fs::read_to_string(path) {
        Ok(contents) => {
            serde_json::from_str(&contents).map_err(|e| format!("AI 설정을 읽을 수 없습니다: {e}"))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(StoredConfig::default()),
        Err(error) => Err(format!("AI 설정 파일을 읽을 수 없습니다: {error}")),
    }
}

fn write_config(app: &AppHandle, config: &StoredConfig) -> Result<(), String> {
    let path = config_path(app)?;
    let temp = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let encoded = serde_json::to_vec_pretty(config)
        .map_err(|e| format!("AI 설정을 저장할 수 없습니다: {e}"))?;
    fs::write(&temp, encoded)
        .map_err(|e| format!("AI 설정 임시 파일을 저장할 수 없습니다: {e}"))?;
    // Windows rename() cannot replace a destination. Keep a small app-owned
    // backup so a failed replacement never destroys the previous settings.
    if path.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(&path, &backup)
            .map_err(|e| format!("기존 AI 설정을 교체할 수 없습니다: {e}"))?;
    }
    match fs::rename(&temp, &path) {
        Ok(()) => {
            let _ = fs::remove_file(&backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup, &path);
            let _ = fs::remove_file(&temp);
            Err(format!("AI 설정을 교체할 수 없습니다: {error}"))
        }
    }
}

#[cfg(windows)]
fn protect(secret: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::null;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: secret.len() as u32,
            pbData: secret.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok = CryptProtectData(
            &input,
            null(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if ok == 0 {
            return Err("Windows DPAPI 암호화에 실패했습니다.".into());
        }
        let value = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(output.pbData as _);
        Ok(value)
    }
}

#[cfg(windows)]
fn unprotect(secret: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: secret.len() as u32,
            pbData: secret.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        let ok = CryptUnprotectData(
            &input,
            null_mut(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if ok == 0 {
            return Err("Windows DPAPI 복호화에 실패했습니다.".into());
        }
        let value = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(output.pbData as _);
        Ok(value)
    }
}

#[cfg(not(windows))]
fn protect(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("FileMoa의 API 키 보호는 Windows DPAPI에서만 지원됩니다.".into())
}

#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Result<Vec<u8>, String> {
    Err("FileMoa의 API 키 보호는 Windows DPAPI에서만 지원됩니다.".into())
}

fn api_key(config: &StoredConfig, id: &str) -> Result<String, String> {
    let entry = config
        .providers
        .get(id)
        .ok_or_else(|| "저장된 API 키가 없습니다.".to_string())?;
    if entry.encrypted_key.is_empty() {
        return Err("저장된 API 키가 없습니다.".into());
    }
    let bytes = unprotect(&entry.encrypted_key)?;
    String::from_utf8(bytes).map_err(|_| "저장된 API 키가 올바르지 않습니다.".into())
}

fn setting_for(id: &str, stored: Option<&StoredProvider>) -> AiProviderSetting {
    let (display_name, default_model, endpoint, registration_url) = provider_defaults(id);
    AiProviderSetting {
        id: id.to_string(),
        display_name: display_name.to_string(),
        model: stored
            .map(|entry| entry.model.clone())
            .filter(|model| !model.is_empty())
            .unwrap_or_else(|| default_model.to_string()),
        key_present: stored
            .map(|entry| !entry.encrypted_key.is_empty())
            .unwrap_or(false),
        enabled: stored.map(|entry| entry.enabled).unwrap_or(true),
        registration_url: registration_url.to_string(),
        endpoint: endpoint.to_string(),
        ollama_models: Vec::new(),
        available: false,
    }
}

async fn ollama_status() -> OllamaStatus {
    let client = match http_client(Duration::from_secs(2)) {
        Ok(client) => client,
        Err(error) => {
            return OllamaStatus {
                detected: false,
                endpoint: OLLAMA_ENDPOINT.into(),
                models: Vec::new(),
                error: Some(error.to_string()),
            }
        }
    };
    let response = match client
        .get(format!("{OLLAMA_ENDPOINT}/api/tags"))
        .send()
        .await
    {
        Ok(response) => response,
        Err(_error) => {
            return OllamaStatus {
                detected: false,
                endpoint: OLLAMA_ENDPOINT.into(),
                models: Vec::new(),
                error: Some("Ollama가 실행 중인지 확인할 수 없습니다.".into()),
            }
        }
    };
    if !response.status().is_success() {
        return OllamaStatus {
            detected: false,
            endpoint: OLLAMA_ENDPOINT.into(),
            models: Vec::new(),
            error: Some(format!("Ollama 응답 오류: {}", response.status())),
        };
    }
    #[derive(Deserialize)]
    struct Tags {
        #[serde(default)]
        models: Vec<Model>,
    }
    #[derive(Deserialize)]
    struct Model {
        name: String,
    }
    match response.json::<Tags>().await {
        Ok(tags) => OllamaStatus {
            detected: true,
            endpoint: OLLAMA_ENDPOINT.into(),
            models: tags.models.into_iter().map(|model| model.name).collect(),
            error: None,
        },
        Err(error) => OllamaStatus {
            detected: false,
            endpoint: OLLAMA_ENDPOINT.into(),
            models: Vec::new(),
            error: Some(format!("Ollama 응답을 읽을 수 없습니다: {error}")),
        },
    }
}

#[tauri::command]
pub async fn detect_ollama() -> Result<OllamaStatus, String> {
    Ok(ollama_status().await)
}

#[tauri::command]
pub async fn get_ai_provider_settings(app: AppHandle) -> Result<Vec<AiProviderSetting>, String> {
    let config = read_config(&app)?;
    let ollama = ollama_status().await;
    let mut result = provider_ids()
        .into_iter()
        .map(|id| {
            let mut setting = setting_for(id, config.providers.get(id));
            if id == "ollama" {
                setting.available = ollama.detected;
                setting.ollama_models = ollama.models.clone();
                if setting.model.is_empty() {
                    setting.model = ollama.models.first().cloned().unwrap_or_default();
                }
            }
            setting
        })
        .collect::<Vec<_>>();
    // Keep the response deterministic even when the provider list changes.
    result.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(result)
}

#[tauri::command]
pub fn save_ai_provider(
    app: AppHandle,
    provider_id: String,
    model: String,
    api_key: Option<String>,
    enabled: bool,
) -> Result<(), String> {
    if !provider_ids().contains(&provider_id.as_str()) {
        return Err("지원하지 않는 AI 공급자입니다.".into());
    }
    let mut config = read_config(&app)?;
    let existing = config.providers.get(&provider_id).cloned();
    let encrypted_key = match api_key {
        Some(key) if !key.trim().is_empty() => protect(key.trim().as_bytes())?,
        _ => existing
            .map(|entry| entry.encrypted_key)
            .unwrap_or_default(),
    };
    config.providers.insert(
        provider_id,
        StoredProvider {
            model: model.trim().to_string(),
            encrypted_key,
            enabled,
        },
    );
    write_config(&app, &config)
}

#[tauri::command]
pub fn remove_ai_provider_key(app: AppHandle, provider_id: String) -> Result<(), String> {
    let mut config = read_config(&app)?;
    if let Some(entry) = config.providers.get_mut(&provider_id) {
        entry.encrypted_key.clear();
    }
    write_config(&app, &config)
}

async fn check_external_provider(id: &str, key: &str) -> Result<(), String> {
    let client = http_client(Duration::from_secs(8))?;
    let response = match id {
        "openai" => {
            client
                .get("https://api.openai.com/v1/models")
                .bearer_auth(key)
                .send()
                .await
        }
        "claude" => {
            client
                .get("https://api.anthropic.com/v1/models")
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
        }
        "gemini" => {
            client
                .get("https://generativelanguage.googleapis.com/v1beta/models")
                .query(&[("key", key)])
                .send()
                .await
        }
        _ => return Err("외부 API 공급자가 아닙니다.".into()),
    }
    .map_err(|_error| "API 서버에 연결할 수 없습니다. 네트워크와 키를 확인하세요.".to_string())?;
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err("API 키가 거부되었습니다. 키를 확인하세요.".into());
    }
    if !status.is_success() {
        return Err(format!("API 연결 테스트가 실패했습니다 (HTTP {status})."));
    }
    Ok(())
}

const AI_BATCH_SIZE: usize = 50;
const REVIEW_THRESHOLD: f32 = 0.65;

fn classification_prompt(files: &[AiFileMetadata], categories: &[String]) -> String {
    let category_text = if categories.is_empty() {
        "기타".to_string()
    } else {
        categories.join(", ")
    };
    let metadata = files
        .iter()
        .map(|file| {
            json!({
                "id": file.id,
                // Defense in depth: callers should send basenames, but never
                // trust a renderer-provided path when building a remote prompt.
                "name": safe_metadata_name(&file.name),
                "extension": file.extension,
                "sizeBytes": file.size_bytes,
                "modifiedUnixSecs": file.modified_unix_secs,
            })
        })
        .collect::<Vec<_>>();
    format!(
        "다음 파일 메타데이터를 안전하게 분류하세요. 파일 내용이나 전체 경로를 추측하지 마세요.\n\n기존 카테고리: {category_text}\n\n파일 목록(JSON): {}\n\n반드시 JSON 배열만 반환하세요. 각 항목은 {{\"id\": 숫자, \"category\": 문자열, \"confidence\": 0과 1 사이 숫자, \"reason\": 짧은 설명, \"isNewCategory\": 불리언}} 형식이어야 합니다. 기존 카테고리에 맞지 않으면 새 카테고리를 제안하되 isNewCategory를 true로 지정하세요.",
        serde_json::to_string(&metadata).unwrap_or_else(|_| "[]".into())
    )
}

fn safe_metadata_name(name: &str) -> String {
    let basename = name.rsplit(['\\', '/']).next().unwrap_or(name);
    basename.chars().take(255).collect()
}

fn response_text(value: &Value, provider_id: &str) -> Result<String, String> {
    let text = match provider_id {
        "openai" => value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str),
        "claude" => value.pointer("/content/0/text").and_then(Value::as_str),
        "gemini" => value
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str),
        "ollama" => value.get("response").and_then(Value::as_str),
        _ => None,
    };
    text.map(str::to_string)
        .ok_or_else(|| "AI 응답에 분류 결과가 없습니다.".into())
}

fn parse_json_array(raw: &str) -> Result<Value, String> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }
    let start = trimmed
        .find('[')
        .ok_or_else(|| "AI 응답에서 JSON 배열을 찾을 수 없습니다.".to_string())?;
    let end = trimmed
        .rfind(']')
        .ok_or_else(|| "AI 응답의 JSON 배열이 완전하지 않습니다.".to_string())?;
    serde_json::from_str(&trimmed[start..=end])
        .map_err(|_| "AI 응답의 분류 JSON 형식이 올바르지 않습니다.".into())
}

fn normalise_suggestions(
    value: Value,
    files: &[AiFileMetadata],
    categories: &[String],
) -> Result<Vec<AiSuggestion>, String> {
    let list = value
        .as_array()
        .or_else(|| value.get("suggestions").and_then(Value::as_array))
        .ok_or_else(|| "AI 응답이 배열 형식이 아닙니다.".to_string())?;
    let existing = categories
        .iter()
        .map(|category| category.trim().to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    let valid_ids = files
        .iter()
        .map(|file| file.id)
        .collect::<std::collections::HashSet<_>>();
    let mut seen = std::collections::HashSet::new();
    let mut suggestions = Vec::new();
    for item in list {
        let Some(id) = item
            .get("id")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
        else {
            continue;
        };
        if !valid_ids.contains(&id) || !seen.insert(id) {
            continue;
        }
        let mut category = item
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("기타")
            .trim()
            .to_string();
        category = crate::planner::sanitize_category(&category, "ko");
        if category.len() > 80 {
            category.truncate(80);
        }
        let confidence = item
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0) as f32;
        let mut reason = item
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("AI가 분류 근거를 제공하지 않았습니다.")
            .trim()
            .to_string();
        if reason.len() > 240 {
            reason.truncate(240);
        }
        let is_new = item
            .get("isNewCategory")
            .and_then(Value::as_bool)
            .unwrap_or(!existing.contains(&category.to_ascii_lowercase()));
        suggestions.push(AiSuggestion {
            id,
            category,
            confidence,
            reason,
            is_new_category: is_new,
            needs_review: confidence < REVIEW_THRESHOLD,
        });
    }
    Ok(suggestions)
}

async fn classify_batch(
    provider_id: &str,
    model: &str,
    key: Option<&str>,
    files: &[AiFileMetadata],
    categories: &[String],
) -> Result<Vec<AiSuggestion>, String> {
    let prompt = classification_prompt(files, categories);
    let client = http_client(Duration::from_secs(45))?;
    let response = match provider_id {
        "openai" => {
            let key = key.ok_or_else(|| "OpenAI API 키가 저장되지 않았습니다.".to_string())?;
            client
                .post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(key)
                .json(&json!({
                    "model": model,
                    "temperature": 0,
                    "messages": [
                        {"role": "system", "content": "당신은 파일명과 메타데이터만 사용하는 안전한 파일 분류기입니다."},
                        {"role": "user", "content": prompt}
                    ]
                }))
                .send()
                .await
        }
        "claude" => {
            let key = key.ok_or_else(|| "Claude API 키가 저장되지 않았습니다.".to_string())?;
            client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .json(&json!({
                    "model": model,
                    "max_tokens": 2048,
                    "temperature": 0,
                    "system": "당신은 파일명과 메타데이터만 사용하는 안전한 파일 분류기입니다.",
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
        }
        "gemini" => {
            let key = key.ok_or_else(|| "Gemini API 키가 저장되지 않았습니다.".to_string())?;
            let encoded_model = model.replace('/', "%2F");
            client
                .post(format!("https://generativelanguage.googleapis.com/v1beta/models/{encoded_model}:generateContent"))
                .query(&[("key", key)])
                .json(&json!({
                    "contents": [{"parts": [{"text": prompt}]}],
                    "generationConfig": {"temperature": 0, "responseMimeType": "application/json"}
                }))
                .send()
                .await
        }
        "ollama" => {
            if model.trim().is_empty() { return Err("Ollama 모델을 먼저 선택하세요.".into()); }
            client
                .post(format!("{OLLAMA_ENDPOINT}/api/generate"))
                .json(&json!({"model": model, "prompt": prompt, "stream": false, "format": "json"}))
                .send()
                .await
        }
        _ => return Err("지원하지 않는 AI 공급자입니다.".into()),
    }
    .map_err(|_| "AI 서버에 연결할 수 없습니다. 연결과 키를 확인하세요.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "AI 분류 요청이 실패했습니다 (HTTP {}).",
            response.status()
        ));
    }
    let payload = response
        .json::<Value>()
        .await
        .map_err(|_| "AI 응답을 읽을 수 없습니다.".to_string())?;
    let text = response_text(&payload, provider_id)?;
    normalise_suggestions(parse_json_array(&text)?, files, categories)
}

#[tauri::command]
pub async fn classify_files(
    app: AppHandle,
    provider_id: String,
    files: Vec<AiFileMetadata>,
    categories: Vec<String>,
) -> Result<AiClassificationResult, String> {
    if !provider_ids().contains(&provider_id.as_str()) {
        return Err("지원하지 않는 AI 공급자입니다.".into());
    }
    if files.is_empty() {
        return Ok(AiClassificationResult {
            provider_id,
            model: String::new(),
            request_count: 0,
            suggestions: Vec::new(),
            proposed_categories: Vec::new(),
        });
    }
    let config = read_config(&app)?;
    let stored = config
        .providers
        .get(&provider_id)
        .ok_or_else(|| "AI 공급자 설정을 먼저 저장하세요.".to_string())?;
    let model = stored.model.trim().to_string();
    if model.is_empty() {
        return Err("AI 모델 ID를 입력하세요.".into());
    }
    let key = if provider_id == "ollama" {
        None
    } else {
        Some(api_key(&config, &provider_id)?)
    };
    let mut request_count = 0;
    let mut suggestions = Vec::new();
    for batch in files.chunks(AI_BATCH_SIZE) {
        request_count += 1;
        suggestions.extend(
            classify_batch(&provider_id, &model, key.as_deref(), batch, &categories).await?,
        );
    }
    suggestions.sort_by_key(|suggestion| suggestion.id);
    let mut proposed_categories = suggestions
        .iter()
        .filter(|suggestion| suggestion.is_new_category)
        .map(|suggestion| suggestion.category.clone())
        .collect::<Vec<_>>();
    proposed_categories.sort();
    proposed_categories.dedup();
    Ok(AiClassificationResult {
        provider_id,
        model,
        request_count,
        suggestions,
        proposed_categories,
    })
}

#[tauri::command]
pub async fn test_ai_provider_connection(
    app: AppHandle,
    provider_id: String,
) -> Result<(), String> {
    if provider_id == "ollama" {
        let status = ollama_status().await;
        return if status.detected {
            Ok(())
        } else {
            Err(status
                .error
                .unwrap_or_else(|| "Ollama가 실행 중이 아닙니다.".into()))
        };
    }
    let config = read_config(&app)?;
    let key = api_key(&config, &provider_id)?;
    check_external_provider(&provider_id, &key).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_four_provider_defaults_without_a_secret() {
        let ids = provider_ids();
        assert_eq!(ids, ["openai", "claude", "gemini", "ollama"]);
        for id in ids {
            let setting = setting_for(id, None);
            assert!(!setting.key_present);
            assert!(!setting.registration_url.is_empty());
        }
    }

    #[test]
    fn serialised_settings_never_contain_an_api_key_field() {
        let config = StoredConfig {
            providers: BTreeMap::from([(
                "openai".into(),
                StoredProvider {
                    model: "test-model".into(),
                    encrypted_key: vec![1, 2, 3],
                    enabled: true,
                },
            )]),
        };
        let json = serde_json::to_string(&config).expect("settings should serialise");
        assert!(!json.contains("test-secret"));
        assert!(json.contains("encrypted_key"));
    }

    #[test]
    fn metadata_prompt_never_contains_a_full_path() {
        let files = vec![AiFileMetadata {
            id: 1,
            name: r"C:\Users\demo\Downloads\receipt.pdf".into(),
            extension: "pdf".into(),
            size_bytes: 12,
            modified_unix_secs: None,
        }];
        let prompt = classification_prompt(&files, &["Documents".into()]);
        assert!(prompt.contains("receipt.pdf"));
        assert!(!prompt.contains(r"C:\Users\demo"));
    }
}
