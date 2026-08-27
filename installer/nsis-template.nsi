!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "MuteGuard"
OutFile "{{output_path}}"
Unicode true
{{#if installer_icon}}
Icon "{{installer_icon}}"
UninstallIcon "{{installer_icon}}"
{{/if}}
{{#if install_mode_per_machine}}
InstallDir "$PROGRAMFILES\MuteGuard"
{{else}}
InstallDir "$LOCALAPPDATA\Programs\MuteGuard"
{{/if}}

{{#if install_mode_per_machine}}
RequestExecutionLevel admin
{{else if install_mode_both}}
RequestExecutionLevel admin
{{else}}
RequestExecutionLevel user
{{/if}}

VIProductVersion "{{version}}.0"
VIAddVersionKey "ProductName" "MuteGuard"
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
VIAddVersionKey "FileDescription" "{{short_description}}"
{{#if publisher}}
VIAddVersionKey "CompanyName" "{{publisher}}"
{{/if}}
{{#if copyright}}
VIAddVersionKey "LegalCopyright" "{{copyright}}"
{{/if}}

!define MUI_ABORTWARNING
{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
{{/if}}
!define MUI_FINISHPAGE_RUN "$INSTDIR\{{main_binary_name}}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch MuteGuard"

{{#if license}}
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
{{#each additional_languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}

Section "Install"
    IfFileExists "$INSTDIR\{{main_binary_name}}" 0 appStoppedForInstall
    ExecWait '"$INSTDIR\{{main_binary_name}}" --exit-all'
    Sleep 300
appStoppedForInstall:
    Delete "$INSTDIR\assets\*.*"
    RMDir "$INSTDIR\assets"
    SetOutPath $INSTDIR
    File "{{main_binary_path}}"
    {{#if installer_icon}}
    File /oname=muteguard.ico "{{installer_icon}}"
    {{/if}}

    {{#each staged_files}}
    SetOutPath "$INSTDIR{{#if this.target_dir}}\{{this.target_dir}}{{/if}}"
    File "{{this.source}}"
    {{/each}}
    Delete "$INSTDIR\*.md"
    SetOutPath $INSTDIR

    WriteUninstaller "$INSTDIR\uninstall.exe"
    CreateDirectory "$SMPROGRAMS\{{start_menu_folder}}"
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\MuteGuard.lnk" "$INSTDIR\{{main_binary_name}}" "" "$INSTDIR\muteguard.ico" 0
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\Uninstall MuteGuard.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\muteguard.ico" 0
    CreateShortcut "$DESKTOP\MuteGuard.lnk" "$INSTDIR\{{main_binary_name}}" "" "$INSTDIR\muteguard.ico" 0

    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "DisplayName" "MuteGuard"
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "DisplayIcon" "$INSTDIR\{{main_binary_name}},0"
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "DisplayVersion" "{{version}}"
    {{#if publisher}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "Publisher" "{{publisher}}"
    {{/if}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "InstallLocation" "$INSTDIR"

    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" "EstimatedSize" "$0"

    {{#if install_webview}}
    {{webview_install_code}}
    {{/if}}
SectionEnd

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

Section "Uninstall"
    IfFileExists "$INSTDIR\{{main_binary_name}}" 0 appStoppedForUninstall
    ExecWait '"$INSTDIR\{{main_binary_name}}" --exit-all'
    Sleep 300
appStoppedForUninstall:
    DeleteRegValue SHCTX "Software\Microsoft\Windows\CurrentVersion\Run" "MuteGuard"
    DeleteRegKey SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}"
    Delete "$DESKTOP\MuteGuard.lnk"
    Delete "$SMPROGRAMS\{{start_menu_folder}}\MuteGuard.lnk"
    Delete "$SMPROGRAMS\{{start_menu_folder}}\Uninstall MuteGuard.lnk"
    RMDir "$SMPROGRAMS\{{start_menu_folder}}"
    Delete "$INSTDIR\assets\*.*"
    RMDir "$INSTDIR\assets"
    Delete "$INSTDIR\*.*"
    RMDir "$INSTDIR"
SectionEnd
