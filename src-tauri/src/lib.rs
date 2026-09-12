mod planner;

use planner::{create_plan, PlanRequest, OrganizationPlan};

#[tauri::command]
fn plan_organization(request: PlanRequest) -> Result<OrganizationPlan, String> {
    create_plan(request)
}

#[tauri::command]
fn execute_organization(plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    planner::execute_plan(plan)
}

#[tauri::command]
fn find_duplicates(root_path: String, include_subfolders: bool) -> Result<Vec<planner::DuplicateGroup>, String> {
    planner::find_duplicates(&root_path, include_subfolders)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![plan_organization, execute_organization, find_duplicates])
        .run(tauri::generate_context!())
        .expect("error while running FileMoa");
}
