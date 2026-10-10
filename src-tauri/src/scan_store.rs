//! What the last scans reported, so the Recycle Bin action can be limited to it.
//!
//! The renderer asks to recycle a list of paths. Only paths that a scan really
//! listed are accepted, they must still look like they did in the scan, and a
//! group of duplicates can never lose its last remaining copy.

use crate::planner::{modified_secs, path_key, DuplicateGroup, LargeFile};
use crate::recycle::RecycleSkip;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    sync::Mutex,
};

/// Far more than any single folder scan produces; a reset beyond it only means
/// the user has to scan again.
const MAX_ENTRIES: usize = 500_000;

#[derive(Clone)]
enum Kind {
    Duplicate(usize),
    Large,
    EmptyFolder,
}

#[derive(Clone)]
struct Entry {
    kind: Kind,
    size: u64,
    modified: Option<u64>,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Entry>,
    groups: HashMap<usize, Vec<String>>,
    next_group: usize,
}

#[derive(Default)]
pub struct ScanStore(Mutex<Inner>);

fn skip(path: &str, reason: &str) -> RecycleSkip {
    RecycleSkip {
        path: path.into(),
        reason: reason.into(),
    }
}

impl ScanStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn make_room(inner: &mut Inner, incoming: usize) {
        if inner.entries.len() + incoming > MAX_ENTRIES {
            inner.entries.clear();
            inner.groups.clear();
        }
    }

    fn stat(path: &str) -> (u64, Option<u64>) {
        fs::metadata(path)
            .map(|metadata| (metadata.len(), modified_secs(&metadata)))
            .unwrap_or((0, None))
    }

    /// Scans of several folders run one after another, so results accumulate;
    /// every entry is re-checked against the disk before it is used.
    pub fn register_duplicates(&self, groups: &[DuplicateGroup]) {
        let mut inner = self.lock();
        Self::make_room(&mut inner, groups.iter().map(|g| g.file_paths.len()).sum());
        for group in groups {
            let id = inner.next_group;
            inner.next_group += 1;
            let mut members = Vec::new();
            for path in &group.file_paths {
                let (size, modified) = Self::stat(path);
                let key = path_key(Path::new(path));
                inner.entries.insert(
                    key.clone(),
                    Entry {
                        kind: Kind::Duplicate(id),
                        size,
                        modified,
                    },
                );
                members.push(key);
            }
            inner.groups.insert(id, members);
        }
    }

    pub fn register_large(&self, files: &[LargeFile]) {
        let mut inner = self.lock();
        Self::make_room(&mut inner, files.len());
        for file in files {
            let key = path_key(Path::new(&file.path));
            // A file that is also in a duplicate group keeps that role, so the
            // "never the last copy" rule cannot be bypassed by a second scan.
            if matches!(
                inner.entries.get(&key),
                Some(Entry {
                    kind: Kind::Duplicate(_),
                    ..
                })
            ) {
                continue;
            }
            let (size, modified) = Self::stat(&file.path);
            inner.entries.insert(
                key,
                Entry {
                    kind: Kind::Large,
                    size,
                    modified,
                },
            );
        }
    }

    pub fn register_empty(&self, folders: &[String]) {
        let mut inner = self.lock();
        Self::make_room(&mut inner, folders.len());
        for folder in folders {
            inner.entries.insert(
                path_key(Path::new(folder)),
                Entry {
                    kind: Kind::EmptyFolder,
                    size: 0,
                    modified: None,
                },
            );
        }
    }

    /// Split a request into what may go to the Recycle Bin and what may not.
    pub fn authorize(&self, requested: &[String]) -> (Vec<String>, Vec<RecycleSkip>) {
        let inner = self.lock();
        let mut allowed: Vec<(String, Entry)> = Vec::new();
        let mut skipped = Vec::new();
        for path in requested {
            let key = path_key(Path::new(path));
            let Some(entry) = inner.entries.get(&key) else {
                skipped.push(skip(
                    path,
                    "검사 결과에 없는 항목이라 거부됨 / refused: not listed by a scan",
                ));
                continue;
            };
            let unchanged = match entry.kind {
                Kind::EmptyFolder => {
                    Path::new(path).is_dir()
                        && fs::read_dir(path)
                            .map(|mut children| children.next().is_none())
                            .unwrap_or(false)
                }
                Kind::Duplicate(_) | Kind::Large => fs::metadata(path)
                    .map(|metadata| {
                        metadata.is_file()
                            && metadata.len() == entry.size
                            && modified_secs(&metadata) == entry.modified
                    })
                    .unwrap_or(false),
            };
            if !unchanged {
                skipped.push(skip(
                    path,
                    "검사 이후 변경되어 건너뜀 / skipped: it changed after the scan",
                ));
                continue;
            }
            allowed.push((path.clone(), entry.clone()));
        }

        // A group may lose every copy but one.
        let requested_keys: HashSet<String> = allowed
            .iter()
            .map(|(path, _)| path_key(Path::new(path)))
            .collect();
        let mut doomed_groups = HashSet::new();
        let touched: HashSet<usize> = allowed
            .iter()
            .filter_map(|(_, entry)| match entry.kind {
                Kind::Duplicate(group) => Some(group),
                _ => None,
            })
            .collect();
        for group in touched {
            let survivors = inner.groups.get(&group).map_or(0, |members| {
                members
                    .iter()
                    .filter(|key| {
                        !requested_keys.contains(*key) && Path::new(key.as_str()).exists()
                    })
                    .count()
            });
            if survivors == 0 {
                doomed_groups.insert(group);
            }
        }
        let mut accepted = Vec::new();
        for (path, entry) in allowed {
            match entry.kind {
                Kind::Duplicate(group) if doomed_groups.contains(&group) => skipped.push(skip(
                    &path,
                    "복사본을 모두 지울 수 없어 거부됨 / refused: at least one copy must stay",
                )),
                _ => accepted.push(path),
            }
        }
        (accepted, skipped)
    }

    /// Forget items that have been recycled.
    pub fn forget(&self, paths: &[String]) {
        let mut inner = self.lock();
        for path in paths {
            let key = path_key(Path::new(path));
            if let Some(Entry {
                kind: Kind::Duplicate(group),
                ..
            }) = inner.entries.remove(&key)
            {
                if let Some(members) = inner.groups.get_mut(&group) {
                    members.retain(|member| member != &key);
                }
            } else {
                inner.entries.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> std::io::Result<tempfile::TempDir> {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        fs::create_dir_all(&base)?;
        tempfile::Builder::new().tempdir_in(base)
    }

    fn group(paths: &[&Path]) -> DuplicateGroup {
        DuplicateGroup {
            hash: "h".into(),
            size_bytes: 4,
            file_paths: paths.iter().map(|p| p.display().to_string()).collect(),
            modified_secs: Vec::new(),
            reclaimable_bytes: 4,
        }
    }

    fn text(path: &Path) -> String {
        path.display().to_string()
    }

    #[test]
    fn only_paths_a_scan_listed_are_authorised() {
        let dir = tempdir().expect("dir");
        let (listed, stranger) = (
            dir.path().join("listed.bin"),
            dir.path().join("stranger.bin"),
        );
        fs::write(&listed, b"data").expect("listed");
        fs::write(&stranger, b"data").expect("stranger");
        let store = ScanStore::default();
        store.register_large(&[LargeFile {
            path: text(&listed),
            size_bytes: 4,
        }]);
        let (accepted, skipped) = store.authorize(&[text(&listed), text(&stranger)]);
        assert_eq!(accepted, vec![text(&listed)]);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].path, text(&stranger));
    }

    #[test]
    fn a_file_that_changed_since_the_scan_is_refused() {
        let dir = tempdir().expect("dir");
        let file = dir.path().join("a.bin");
        fs::write(&file, b"data").expect("file");
        let store = ScanStore::default();
        store.register_large(&[LargeFile {
            path: text(&file),
            size_bytes: 4,
        }]);
        fs::write(&file, b"data that is now longer").expect("edit");
        let (accepted, skipped) = store.authorize(&[text(&file)]);
        assert!(accepted.is_empty());
        assert!(skipped[0].reason.contains("changed"));
    }

    #[test]
    fn the_last_copy_of_a_duplicate_group_is_never_released() {
        let dir = tempdir().expect("dir");
        let paths: Vec<_> = ["a.bin", "b.bin", "c.bin"]
            .iter()
            .map(|n| dir.path().join(n))
            .collect();
        for path in &paths {
            fs::write(path, b"same").expect("copy");
        }
        let store = ScanStore::default();
        store.register_duplicates(&[group(&[&paths[0], &paths[1], &paths[2]])]);

        // Two of three: fine.
        let (accepted, skipped) = store.authorize(&[text(&paths[0]), text(&paths[1])]);
        assert_eq!((accepted.len(), skipped.len()), (2, 0));

        // All three: refused entirely, nothing may be recycled.
        let all: Vec<_> = paths.iter().map(|p| text(p)).collect();
        let (accepted, skipped) = store.authorize(&all);
        assert!(accepted.is_empty(), "{accepted:?}");
        assert_eq!(skipped.len(), 3);
        assert!(skipped[0].reason.contains("one copy"));
    }

    #[test]
    fn copies_that_were_already_removed_do_not_count_as_survivors() {
        let dir = tempdir().expect("dir");
        let (a, b) = (dir.path().join("a.bin"), dir.path().join("b.bin"));
        fs::write(&a, b"same").expect("a");
        fs::write(&b, b"same").expect("b");
        let store = ScanStore::default();
        store.register_duplicates(&[group(&[&a, &b])]);
        // `a` is recycled first; afterwards `b` is the last copy.
        fs::remove_file(&a).expect("remove a");
        store.forget(&[text(&a)]);
        let (accepted, skipped) = store.authorize(&[text(&b)]);
        assert!(accepted.is_empty());
        assert_eq!(skipped.len(), 1);
    }

    #[test]
    fn a_later_large_file_scan_does_not_lift_the_last_copy_rule() {
        let dir = tempdir().expect("dir");
        let (a, b) = (dir.path().join("a.bin"), dir.path().join("b.bin"));
        fs::write(&a, b"same").expect("a");
        fs::write(&b, b"same").expect("b");
        let store = ScanStore::default();
        store.register_duplicates(&[group(&[&a, &b])]);
        store.register_large(&[
            LargeFile {
                path: text(&a),
                size_bytes: 4,
            },
            LargeFile {
                path: text(&b),
                size_bytes: 4,
            },
        ]);
        let (accepted, skipped) = store.authorize(&[text(&a), text(&b)]);
        assert!(
            accepted.is_empty(),
            "both copies must still be protected as a group"
        );
        assert_eq!(skipped.len(), 2);
    }

    #[test]
    fn an_empty_folder_that_gained_a_file_is_refused() {
        let dir = tempdir().expect("dir");
        let folder = dir.path().join("empty");
        fs::create_dir_all(&folder).expect("folder");
        let store = ScanStore::default();
        store.register_empty(&[text(&folder)]);
        assert_eq!(store.authorize(&[text(&folder)]).0.len(), 1);
        fs::write(folder.join("new.txt"), b"x").expect("file appears");
        let (accepted, skipped) = store.authorize(&[text(&folder)]);
        assert!(accepted.is_empty());
        assert_eq!(skipped.len(), 1);
    }
}
