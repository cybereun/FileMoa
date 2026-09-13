# Desktop Command Contract

The React user interface communicates with the local core only through these commands.
No command transmits file metadata to a remote service.

| Command | Input | Output | Safety rule |
|---|---|---|---|
| `create_plan` | Root path, subfolder toggle | OrganizationPlan | No writes |
| `plan_multiple_organizations` | One request per selected root | OrganizationPlan | Roots are isolated; no cross-root moves |
| `execute_plan` | Plan ID, explicit approval | OrganizationRun | Executes only proposed actions |
| `preview_undo` | Run ID | OrganizationPlan | No writes |
| `execute_undo` | Run ID, explicit approval | OrganizationRun | Never overwrites |
| `list_history` | None | OrganizationRun list | Read only |
| `save_category` | CustomCategory | CustomCategory | Local settings only |
| `find_duplicates` | Root path, subfolder toggle | DuplicateGroup list | Read only |
| `find_large_files` | Root path, subfolder toggle, minimum bytes | LargeFile list | Read only |
| `find_empty_folders` | Root path, subfolder toggle | Folder path list | Read only |
| `recycle_files` | Explicit file/folder paths | Recycled paths | Confirmation required; Recycle Bin only |
| `check_for_update` | None | Update metadata or none | Explicit user action only |
