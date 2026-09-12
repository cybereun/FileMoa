use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest { pub root_path: String, pub include_subfolders: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAction { pub source_path: String, pub destination_path: String, pub category: String, pub reason: String, pub status: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlan { pub root_path: String, pub actions: Vec<PlannedAction>, pub exclusions: Vec<String>, pub errors: Vec<String> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup { pub hash: String, pub size_bytes: u64, pub file_paths: Vec<String>, pub reclaimable_bytes: u64 }

fn category_for(path: &Path) -> (&'static str, &'static str) {
    let ext = path.extension().and_then(|v| v.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => ("문서/PDF", "PDF document"), "doc"|"docx" => ("문서/Word", "Word document"),
        "xls"|"xlsx"|"csv" => ("문서/Excel", "Spreadsheet"), "ppt"|"pptx" => ("문서/PowerPoint", "Presentation"),
        "hwp"|"hwpx" => ("문서/한글", "Hangul document"), "txt"|"md"|"rtf" => ("문서/텍스트", "Text document"),
        "jpg"|"jpeg"|"png"|"gif"|"webp"|"bmp"|"tiff"|"heic"|"avif" => ("이미지", "Image"),
        "mp4"|"mov"|"avi"|"mkv"|"wmv"|"webm" => ("동영상", "Video"),
        "mp3"|"wav"|"flac"|"aac"|"m4a"|"ogg"|"wma" => ("오디오", "Audio"),
        "zip"|"rar"|"7z"|"tar"|"gz"|"bz2"|"xz" => ("압축파일", "Archive"),
        "exe"|"msi"|"msix"|"appx"|"appxbundle" => ("설치파일", "Installer"),
        "js"|"ts"|"tsx"|"html"|"css"|"py"|"rs"|"java"|"json"|"yaml"|"yml"|"toml"|"sql" => ("개발", "Development file"),
        _ => ("기타", "Other file")
    }
}
fn is_protected(path: &Path) -> bool { let s = path.to_string_lossy().to_ascii_lowercase(); ["c:\\windows", "c:\\program files", "appdata", "node_modules", "\\.git"].iter().any(|p| s.contains(p)) }
fn is_project_dir(path: &Path) -> bool { ["package.json", "pyproject.toml", "cargo.toml", "go.mod", "pom.xml", ".git"].iter().any(|m| path.join(m).exists()) }
fn collision_safe(destination: PathBuf) -> PathBuf { if !destination.exists() { return destination; } let stem=destination.file_stem().and_then(|v|v.to_str()).unwrap_or("file"); let ext=destination.extension().and_then(|v|v.to_str()).unwrap_or(""); for n in 1..10000 { let name=if ext.is_empty(){format!("{stem} ({n})")}else{format!("{stem} ({n}).{ext}")}; let candidate=destination.with_file_name(name); if !candidate.exists(){return candidate;} } destination }
pub fn create_plan(request: PlanRequest) -> Result<OrganizationPlan, String> {
 let root=PathBuf::from(&request.root_path); if !root.is_dir(){return Err("선택한 폴더를 찾을 수 없습니다.".into())} if is_protected(&root){return Err("보호된 시스템 또는 개발 경로는 정리할 수 없습니다.".into())}
 let mut actions=vec![]; let mut exclusions=vec![]; let iter=if request.include_subfolders { WalkDir::new(&root).min_depth(1).into_iter() } else { WalkDir::new(&root).max_depth(1).min_depth(1).into_iter() };
 for entry in iter.filter_map(Result::ok) { let path=entry.path(); if entry.file_type().is_dir(){if path!=root && is_project_dir(path){exclusions.push(format!("프로젝트 보호: {}",path.display()));} continue;} if !entry.file_type().is_file(){continue;} let name=path.file_name().and_then(|v|v.to_str()).unwrap_or(""); if name.starts_with('.')||["crdownload","part","tmp"].iter().any(|e|name.ends_with(e)){exclusions.push(format!("임시/숨김 파일: {}",path.display()));continue;} let (category,reason)=category_for(path); let dest=collision_safe(root.join(category).join(path.file_name().unwrap())); actions.push(PlannedAction{source_path:path.display().to_string(),destination_path:dest.display().to_string(),category:category.into(),reason:reason.into(),status:"proposed".into()}); }
 Ok(OrganizationPlan{root_path:request.root_path,actions,exclusions,errors:vec![]})
}
pub fn execute_plan(mut plan: OrganizationPlan)->Result<OrganizationPlan,String>{ for action in &mut plan.actions { let from=Path::new(&action.source_path); let to=Path::new(&action.destination_path); if !from.exists(){action.status="skipped".into();action.reason="Source file no longer exists".into();continue;} if let Some(parent)=to.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;} fs::rename(from,to).map_err(|e|e.to_string())?;action.status="executed".into(); } Ok(plan) }
pub fn find_duplicates(root_path:&str, include_subfolders:bool)->Result<Vec<DuplicateGroup>,String>{ let root=Path::new(root_path); let mut sizes:HashMap<u64,Vec<PathBuf>>=HashMap::new(); let iter=if include_subfolders{WalkDir::new(root).into_iter()}else{WalkDir::new(root).max_depth(1).into_iter()}; for entry in iter.filter_map(Result::ok){if entry.file_type().is_file(){if let Ok(meta)=entry.metadata(){sizes.entry(meta.len()).or_default().push(entry.into_path());}}} let mut groups=vec![];for(size,files)in sizes{if files.len()<2{continue}let mut hashes:HashMap<String,Vec<String>>=HashMap::new();for file in files{if let Ok(data)=fs::read(&file){let hash=format!("{:x}",Sha256::digest(data));hashes.entry(hash).or_default().push(file.display().to_string());}}for(hash,paths)in hashes{if paths.len()>1{groups.push(DuplicateGroup{hash,size_bytes:size,reclaimable_bytes:size*(paths.len()as u64-1),file_paths:paths});}}}Ok(groups)}

#[cfg(test)] mod tests { use super::*; #[test] fn plan_classifies_hangul_and_avoids_collisions(){let d=std::env::current_dir().unwrap().join("target/filemoa-planner-test");let _=fs::remove_dir_all(&d);fs::create_dir_all(&d).unwrap();fs::write(d.join("a.hwp"),"x").unwrap();fs::create_dir_all(d.join("문서/한글")).unwrap();fs::write(d.join("문서/한글/a.hwp"),"y").unwrap();let plan=create_plan(PlanRequest{root_path:d.display().to_string(),include_subfolders:false}).unwrap();assert_eq!(plan.actions.len(),1);assert!(plan.actions[0].destination_path.contains("a (1).hwp"));let _=fs::remove_dir_all(&d);}}
