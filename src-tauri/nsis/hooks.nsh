; FileMoa shell integration. The installer runs in current-user mode, so all
; registrations stay under HKCU and never require administrator elevation.

!define FILEMOA_SHELL_LABEL "FileMoa로 정리 / Organize with FileMoa"

!macro FILEMOA_WRITE_SHELL_VERB ROOT_PATH PATH_TOKEN
  WriteRegStr HKCU "${ROOT_PATH}" "MUIVerb" "${FILEMOA_SHELL_LABEL}"
  WriteRegStr HKCU "${ROOT_PATH}" "Icon" "$INSTDIR\FileMoa.exe"
  WriteRegStr HKCU "${ROOT_PATH}" "Position" "Top"
  WriteRegStr HKCU "${ROOT_PATH}\command" "" '"$INSTDIR\FileMoa.exe" --organize "${PATH_TOKEN}"'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; A folder itself, a folder background, and a drive root are all useful
  ; entry points. %1 is the selected item; %V is the current background path.
  !insertmacro FILEMOA_WRITE_SHELL_VERB "Software\Classes\Directory\shell\FileMoa" "%1"
  !insertmacro FILEMOA_WRITE_SHELL_VERB "Software\Classes\Directory\Background\shell\FileMoa" "%V"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\FileMoa" "NoWorkingDirectory" ""
  !insertmacro FILEMOA_WRITE_SHELL_VERB "Software\Classes\DesktopBackground\shell\FileMoa" "%V"
  WriteRegStr HKCU "Software\Classes\DesktopBackground\shell\FileMoa" "NoWorkingDirectory" ""
  !insertmacro FILEMOA_WRITE_SHELL_VERB "Software\Classes\Drive\shell\FileMoa" "%1"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\Directory\shell\FileMoa"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\FileMoa"
  DeleteRegKey HKCU "Software\Classes\DesktopBackground\shell\FileMoa"
  DeleteRegKey HKCU "Software\Classes\Drive\shell\FileMoa"
!macroend
