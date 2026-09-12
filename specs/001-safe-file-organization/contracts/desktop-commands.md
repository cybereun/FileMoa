# Desktop Command Contract

The React user interface communicates with the local core only through these commands.
No command transmits file metadata to a remote service.

| Command | Input | Output | Safety rule |
|---|---|---|---|
| `create_plan` | Root path, subfolder toggle | OrganizationPlan | No writes |
| `execute_plan` | Plan ID, explicit approval | OrganizationRun | Executes only proposed actions |
| `preview_undo` | Run ID | OrganizationPlan | No writes |
| `execute_undo` | Run ID, explicit approval | OrganizationRun | Never overwrites |
| `list_history` | None | OrganizationRun list | Read only |
| `save_category` | CustomCategory | CustomCategory | Local settings only |
| `find_duplicates` | Root path, subfolder toggle | DuplicateGroup list | Read only |
| `check_for_update` | None | Update metadata or none | Explicit user action only |
