; Crosshair Studio — NSIS installer hooks.
; Ensures a clean uninstall: the optional "launch at Windows startup" entry is
; written by the app itself in HKCU\...\Run, so the uninstaller removes it.

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "CrosshairStudio"
!macroend
