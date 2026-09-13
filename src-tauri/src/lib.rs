mod ai;
mod planner;
mod recycle;

use planner::{create_plan, OrganizationPlan, PlanRequest};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnownFolder {
    id: String,
    path: String,
}

#[tauri::command]
fn known_folders(app: AppHandle) -> Vec<KnownFolder> {
    let locations = [
        ("desktop", app.path().desktop_dir()),
        ("downloads", app.path().download_dir()),
        ("documents", app.path().document_dir()),
        ("pictures", app.path().picture_dir()),
        ("videos", app.path().video_dir()),
        ("music", app.path().audio_dir()),
    ];
    locations
        .into_iter()
        .filter_map(|(id, path)| {
            let path = path.ok()?;
            path.is_dir().then(|| KnownFolder {
                id: id.into(),
                path: path.display().to_string(),
            })
        })
        .collect()
}

fn startup_candidate<I>(mut arguments: I) -> Result<Option<PathBuf>, String>
where
    I: Iterator<Item = std::ffi::OsString>,
{
    while let Some(argument) = arguments.next() {
        if !argument
            .to_string_lossy()
            .eq_ignore_ascii_case("--organize")
        {
            continue;
        }
        let Some(candidate) = arguments.next() else {
            return Err(
                "FileMoa 셸 실행 인자에 폴더 경로가 없습니다. / The shell launch did not include a folder path."
                    .into(),
            );
        };
        return Ok(Some(PathBuf::from(candidate)));
    }
    Ok(None)
}

/// Return the single folder passed by an Explorer/Desktop shell verb. The
/// explicit marker prevents unrelated runtime arguments from being treated as
/// a folder, and invalid paths fail before the planner can perform any work.
#[tauri::command]
fn startup_path() -> Result<Option<String>, String> {
    let Some(path) = startup_candidate(std::env::args_os().skip(1))? else {
        return Ok(None);
    };
    if !path.is_dir() {
        return Err(format!(
            "셸에서 전달된 폴더를 찾을 수 없습니다: {} / The shell folder is not available: {}",
            path.display(),
            path.display()
        ));
    }
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
fn plan_organization(request: PlanRequest) -> Result<OrganizationPlan, String> {
    create_plan(request)
}

#[tauri::command]
fn plan_multiple_organizations(requests: Vec<PlanRequest>) -> Result<OrganizationPlan, String> {
    planner::create_multi_plan(requests)
}

#[tauri::command]
fn execute_organization(plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    planner::execute_plan(plan)
}

#[tauri::command]
fn undo_organization(plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    planner::undo_plan(plan)
}

#[tauri::command]
fn find_duplicates(
    root_path: String,
    include_subfolders: bool,
) -> Result<Vec<planner::DuplicateGroup>, String> {
    planner::find_duplicates(&root_path, include_subfolders)
}

#[tauri::command]
fn find_large_files(
    root_path: String,
    include_subfolders: bool,
    minimum_size_bytes: u64,
) -> Result<Vec<planner::LargeFile>, String> {
    planner::find_large_files(&root_path, include_subfolders, minimum_size_bytes)
}

#[tauri::command]
fn find_empty_folders(root_path: String, include_subfolders: bool) -> Result<Vec<String>, String> {
    planner::find_empty_folders(&root_path, include_subfolders)
}

#[tauri::command]
fn recycle_files(paths: Vec<String>) -> Result<Vec<String>, String> {
    recycle::recycle_files(paths)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            known_folders,
            startup_path,
            plan_organization,
            plan_multiple_organizations,
            execute_organization,
            undo_organization,
            find_duplicates,
            find_large_files,
            find_empty_folders,
            recycle_files,
            ai::detect_ollama,
            ai::get_ai_provider_settings,
            ai::save_ai_provider,
            ai::remove_ai_provider_key,
            ai::test_ai_provider_connection,
            ai::classify_files,
            ai::apply_ai_suggestions
        ])
        .run(tauri::generate_context!())
        .expect("error while running FileMoa");
}

#[cfg(test)]
mod tests {
    use super::startup_candidate;
    use std::ffi::OsString;

    #[test]
    fn shell_parser_requires_explicit_marker() {
        let args = vec![OsString::from(r"C:\not-a-folder")].into_iter();
        assert!(startup_candidate(args).expect("parse").is_none());
    }

    #[test]
    fn shell_parser_preserves_unicode_and_spaces() {
        let expected = r"C:\Users\demo\바탕 화면\작업 폴더";
        let args = vec![OsString::from("--organize"), OsString::from(expected)].into_iter();
        let actual = startup_candidate(args).expect("parse").expect("path");
        assert_eq!(actual.to_string_lossy(), expected);
    }

    #[test]
    fn shell_parser_rejects_missing_path_argument() {
        let args = vec![OsString::from("--organize")].into_iter();
        assert!(startup_candidate(args).is_err());
    }
}
