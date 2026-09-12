use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCategory { pub name: String, pub extensions: Vec<String>, pub enabled: bool }
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest { pub root_path: String, pub include_subfolders: bool, #[serde(default)] pub custom_categories: Vec<CustomCategory> }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedAction { pub source_path: String, pub destination_path: String, pub category: String, pub reason: String, pub status: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlan { pub root_path: String, pub actions: Vec<PlannedAction>, pub exclusions: Vec<String>, pub errors: Vec<String> }
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup { pub hash: String, pub size_bytes: u64, pub file_paths: Vec<String>, pub reclaimable_bytes: u64 }

fn builtin(ext:&str)->(&'static str,&'static str){match ext {
"pdf"=> ("문서/PDF","PDF document"),"doc"|"docx"=> ("문서/Word","Word document"),"xls"|"xlsx"|"csv"=> ("문서/Excel","Spreadsheet"),"ppt"|"pptx"=> ("문서/PowerPoint","Presentation"),"hwp"|"hwpx"=> ("문서/한글","Hangul document"),"txt"|"md"|"rtf"=> ("문서/텍스트","Text document"),"jpg"|"jpeg"|"png"|"gif"|"webp"|"bmp"|"tiff"|"heic"|"avif"=> ("이미지","Image"),"mp4"|"mov"|"avi"|"mkv"|"wmv"|"webm"=> ("동영상","Video"),"mp3"|"wav"|"flac"|"aac"|"m4a"|"ogg"|"wma"=> ("오디오","Audio"),"zip"|"rar"|"7z"|"tar"|"gz"|"bz2"|"xz"=> ("압축파일","Archive"),"exe"|"msi"|"msix"|"appx"|"appxbundle"=> ("설치파일","Installer"),"js"|"ts"|"tsx"|"html"|"css"|"py"|"rs"|"java"|"json"|"yaml"|"yml"|"toml"|"sql"=> ("개발","Development file"),_=> ("기타","Other file")}}
fn protected(path:&Path)->bool{let s=path.to_string_lossy().to_ascii_lowercase();["c:\\windows","c:\\program files","appdata","node_modules","\\.git"].iter().any(|p|s.contains(p))}
fn project(path:&Path)->bool{["package.json","pyproject.toml","cargo.toml","go.mod","pom.xml",".git"].iter().any(|m|path.join(m).exists())}
fn safe(dest:PathBuf)->PathBuf{if !dest.exists(){return dest}let stem=dest.file_stem().and_then(|x|x.to_str()).unwrap_or("file");let ext=dest.extension().and_then(|x|x.to_str()).unwrap_or("");for n in 1..10000{let name=if ext.is_empty(){format!("{stem} ({n})")}else{format!("{stem} ({n}).{ext}")};let c=dest.with_file_name(name);if !c.exists(){return c}}dest}
fn category(path:&Path, custom:&[CustomCategory])->(String,String){let ext=path.extension().and_then(|x|x.to_str()).unwrap_or("").to_ascii_lowercase();if let Some(rule)=custom.iter().find(|r|r.enabled&&r.extensions.iter().any(|e|e.trim_start_matches('.').eq_ignore_ascii_case(&ext))){return(rule.name.clone(),"Custom category".into())}let (a,b)=builtin(&ext);(a.into(),b.into())}
pub fn create_plan(req:PlanRequest)->Result<OrganizationPlan,String>{let root=PathBuf::from(&req.root_path);if !root.is_dir(){return Err("선택한 폴더를 찾을 수 없습니다.".into())}if protected(&root)||project(&root){return Err("보호된 시스템 또는 개발 프로젝트 경로는 정리할 수 없습니다.".into())}let mut actions=vec![];let mut exclusions=vec![];let walker=if req.include_subfolders{WalkDir::new(&root).min_depth(1)}else{WalkDir::new(&root).min_depth(1).max_depth(1)};for entry in walker.into_iter().filter_map(Result::ok){if entry.file_type().is_dir(){if entry.path()!=root&&project(entry.path()){exclusions.push(format!("프로젝트 보호: {}",entry.path().display()));}continue}if !entry.file_type().is_file(){continue}let path=entry.path();if path.ancestors().skip(1).any(|p|p!=root&&project(p)){continue}let name=path.file_name().and_then(|x|x.to_str()).unwrap_or("");if name.starts_with('.')||["crdownload","part","tmp"].iter().any(|x|name.ends_with(x)){exclusions.push(format!("임시/숨김 파일: {}",path.display()));continue}let(cat,reason)=category(path,&req.custom_categories);let dest=safe(root.join(&cat).join(path.file_name().unwrap()));actions.push(PlannedAction{source_path:path.display().to_string(),destination_path:dest.display().to_string(),category:cat,reason,status:"proposed".into()});}Ok(OrganizationPlan{root_path:req.root_path,actions,exclusions,errors:vec![]})}
pub fn execute_plan(mut plan:OrganizationPlan)->Result<OrganizationPlan,String>{for a in &mut plan.actions{let from=Path::new(&a.source_path);let to=Path::new(&a.destination_path);if !from.exists(){a.status="skipped".into();continue}if let Some(parent)=to.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?}match fs::rename(from,to){Ok(_)=>a.status="executed".into(),Err(e)=>{a.status="failed".into();a.reason=e.to_string()}}}Ok(plan)}
pub fn undo_plan(mut plan:OrganizationPlan)->Result<OrganizationPlan,String>{for a in &mut plan.actions{if a.status!="executed"{continue}let from=Path::new(&a.destination_path);let to=Path::new(&a.source_path);if from.exists()&&!to.exists(){if let Some(parent)=to.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?}fs::rename(from,to).map_err(|e|e.to_string())?;a.status="undone".into()}else{a.status="undo_conflict".into()}}Ok(plan)}
pub fn find_duplicates(root:&str,sub:bool)->Result<Vec<DuplicateGroup>,String>{let mut sizes:HashMap<u64,Vec<PathBuf>>=HashMap::new();let walk=if sub{WalkDir::new(root)}else{WalkDir::new(root).max_depth(1)};for e in walk.into_iter().filter_map(Result::ok){if e.file_type().is_file(){if let Ok(m)=e.metadata(){sizes.entry(m.len()).or_default().push(e.into_path())}}}let mut out=vec![];for(size,files)in sizes{if files.len()<2{continue}let mut hashes:HashMap<String,Vec<String>>=HashMap::new();for f in files{if let Ok(b)=fs::read(&f){hashes.entry(format!("{:x}",Sha256::digest(b))).or_default().push(f.display().to_string())}}for(hash,paths)in hashes{if paths.len()>1{out.push(DuplicateGroup{hash,size_bytes:size,reclaimable_bytes:size*(paths.len()as u64-1),file_paths:paths})}}}Ok(out)}
