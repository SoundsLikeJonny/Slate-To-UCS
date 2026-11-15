; The Slate-To-UCS NSIS installer.
;
; Built by packaging/windows/build-nsis.ps1, which passes every path with /D so this file
; holds no absolute path. The MSI is the primary artefact; this exists because an .exe
; installer is what many recordists expect to double-click, and because it can install
; per-user without elevation when the owner asks for that.
;
; Keep the uninstall registry keys and the install layout identical to slate.wxs. Two
; installers that disagree about where files live produce two uninstallers that each
; remove half of the other's work.

Unicode true
ManifestSupportedOS all
ManifestDPIAware true

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "WordFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SLATE_EXE
  !error "SLATE_EXE is required: pass /DSLATE_EXE=<path>"
!endif
!ifndef REAPER_DLL
  !error "REAPER_DLL is required: pass /DREAPER_DLL=<path>"
!endif
!ifndef SCRIPTS_DIR
  !error "SCRIPTS_DIR is required: pass /DSCRIPTS_DIR=<path>"
!endif
!ifndef ICON_PATH
  !define ICON_PATH ""
!endif
!ifndef OUT_FILE
  !define OUT_FILE "Slate-To-UCS-${VERSION}-x64-setup.exe"
!endif

Name "Slate-To-UCS"
OutFile "${OUT_FILE}"
InstallDir "$PROGRAMFILES64\Slate-To-UCS"
InstallDirRegKey HKLM "Software\Slate-To-UCS" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
BrandingText "Slate-To-UCS ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Slate-To-UCS"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}.0"
VIAddVersionKey "CompanyName" "Jon Evans"
VIAddVersionKey "FileDescription" "Slate-To-UCS installer"
VIAddVersionKey "LegalCopyright" "MIT"

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\slate.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open Slate-To-UCS"
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\README.md"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Read the README"
!if "${ICON_PATH}" != ""
  !define MUI_ICON "${ICON_PATH}"
  !define MUI_UNICON "${ICON_PATH}"
!endif

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; Refuse a downgrade rather than silently replacing a newer install, matching the MSI's
; MajorUpgrade rule.
!macro CheckVersion
  ReadRegStr $0 HKLM "Software\Slate-To-UCS" "Version"
  ${If} $0 != ""
    ${VersionCompare} "$0" "${VERSION}" $1
    ${If} $1 == 1
      MessageBox MB_ICONSTOP|MB_OK "Slate-To-UCS $0 is already installed. Uninstall it before installing ${VERSION}."
      Abort
    ${EndIf}
  ${EndIf}
!macroend

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP|MB_OK "Slate-To-UCS needs 64-bit Windows."
    Abort
  ${EndIf}
  SetRegView 64
  !insertmacro CheckVersion
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

Section "Slate-To-UCS" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  SetOverwrite on

  File "${SLATE_EXE}"
  File "${REAPER_DLL}"
  File "..\..\LICENSE"
  File "..\..\README.md"

  SetOutPath "$INSTDIR\Scripts\Slate-To-UCS"
  File "${SCRIPTS_DIR}\lib\slate_api.lua"
  File "${SCRIPTS_DIR}\slate_tag_selected.lua"
  File "${SCRIPTS_DIR}\slate_tag_project.lua"
  File "${SCRIPTS_DIR}\slate_render_pattern.lua"
  File "${SCRIPTS_DIR}\slate_open_app.lua"

  ; The sidecar is a onedir tree, so it is copied wholesale when the caller supplies one.
  !ifdef SIDECAR_DIR
  SetOutPath "$INSTDIR\ttucs"
  File /r "${SIDECAR_DIR}\*.*"
  !endif

  CreateDirectory "$SMPROGRAMS\Slate-To-UCS"
  CreateShortcut "$SMPROGRAMS\Slate-To-UCS\Slate-To-UCS.lnk" "$INSTDIR\slate.exe" "" "$INSTDIR\slate.exe" 0
  CreateShortcut "$SMPROGRAMS\Slate-To-UCS\Uninstall.lnk" "$INSTDIR\uninstall.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "Software\Slate-To-UCS" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Slate-To-UCS" "Version" "${VERSION}"

  ; Add/Remove Programs, with the same display name the MSI uses.
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "DisplayName" "Slate-To-UCS"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "Publisher" "Jon Evans"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "DisplayIcon" "$INSTDIR\slate.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS" "EstimatedSize" "$0"
SectionEnd

Section "Uninstall"
  ; Only what this installer wrote. A user's REAPER integration lives in their profile and
  ; is removed by the application's own reversible action, not here.
  Delete "$INSTDIR\slate.exe"
  Delete "$INSTDIR\slate_reaper.dll"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\Scripts\Slate-To-UCS\lib\slate_api.lua"
  Delete "$INSTDIR\Scripts\Slate-To-UCS\slate_tag_selected.lua"
  Delete "$INSTDIR\Scripts\Slate-To-UCS\slate_tag_project.lua"
  Delete "$INSTDIR\Scripts\Slate-To-UCS\slate_render_pattern.lua"
  Delete "$INSTDIR\Scripts\Slate-To-UCS\slate_open_app.lua"
  RMDir "$INSTDIR\Scripts\Slate-To-UCS\lib"
  RMDir "$INSTDIR\Scripts\Slate-To-UCS"
  RMDir "$INSTDIR\Scripts"
  RMDir /r "$INSTDIR\ttucs"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\Slate-To-UCS\Slate-To-UCS.lnk"
  Delete "$SMPROGRAMS\Slate-To-UCS\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Slate-To-UCS"

  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Slate-To-UCS"
  DeleteRegKey HKLM "Software\Slate-To-UCS"
SectionEnd
