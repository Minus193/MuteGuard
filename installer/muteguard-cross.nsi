!include "MUI2.nsh"
!include "FileFunc.nsh"

!ifndef APP_DIR
    !error "APP_DIR must point to the prepared portable application directory"
!endif
!ifndef OUTPUT_FILE
    !error "OUTPUT_FILE must contain the installer output path"
!endif
!ifndef APP_ICON
    !error "APP_ICON must point to muteguard.ico"
!endif
!ifndef VERSION
    !define VERSION "1.1.1"
!endif

Name "MuteGuard"
OutFile "${OUTPUT_FILE}"
Unicode true
SetCompressor /SOLID lzma
InstallDir "$LOCALAPPDATA\Programs\MuteGuard"
RequestExecutionLevel user

Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "MuteGuard"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "MuteGuard Setup"
VIAddVersionKey "CompanyName" "MuteGuard"
VIAddVersionKey "LegalCopyright" "Apache-2.0 licensed"

!define MUI_ABORTWARNING
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!define MUI_FINISHPAGE_RUN "$INSTDIR\muteguard.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch MuteGuard"

!define WEBVIEW2_CLIENT_KEY "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!define WEBVIEW2_BOOTSTRAPPER_URL "https://go.microsoft.com/fwlink/p/?LinkId=2124703"

!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function EnsureWebView2
    SetRegView 32
    ClearErrors
    ReadRegStr $0 HKLM "${WEBVIEW2_CLIENT_KEY}" "pv"
    IfErrors checkCurrentUser
    StrCmp $0 "" checkCurrentUser
    StrCmp $0 "0.0.0.0" checkCurrentUser runtimeReady

checkCurrentUser:
    ClearErrors
    ReadRegStr $0 HKCU "${WEBVIEW2_CLIENT_KEY}" "pv"
    IfErrors runtimeMissing
    StrCmp $0 "" runtimeMissing
    StrCmp $0 "0.0.0.0" runtimeMissing runtimeReady

runtimeMissing:
    MessageBox MB_YESNO|MB_ICONQUESTION \
        "MuteGuard Settings requires Microsoft Edge WebView2 Runtime, which was not detected.$\r$\n$\r$\nDownload and install it now from Microsoft?" \
        IDYES installRuntime IDNO runtimeSkipped

installRuntime:
    DetailPrint "Downloading Microsoft Edge WebView2 Runtime bootstrapper..."
    NSISdl::download /TIMEOUT=60000 "${WEBVIEW2_BOOTSTRAPPER_URL}" "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    Pop $0
    StrCmp $0 "success" runRuntimeInstaller downloadFailed

runRuntimeInstaller:
    DetailPrint "Installing Microsoft Edge WebView2 Runtime..."
    ExecWait '"$TEMP\MicrosoftEdgeWebview2Setup.exe" /silent /install' $0
    Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    IntCmp $0 0 runtimeReady installFailed installFailed

downloadFailed:
    Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    MessageBox MB_OK|MB_ICONEXCLAMATION \
        "WebView2 could not be downloaded ($0). MuteGuard background controls will still work, but Settings requires WebView2 Runtime."
    Goto runtimeReady

installFailed:
    MessageBox MB_OK|MB_ICONEXCLAMATION \
        "WebView2 setup returned error $0. MuteGuard background controls will still work, but Settings requires WebView2 Runtime."
    Goto runtimeReady

runtimeSkipped:
    MessageBox MB_OK|MB_ICONINFORMATION \
        "MuteGuard background controls will work without WebView2. Install WebView2 Runtime before opening Settings."

runtimeReady:
FunctionEnd

Section "Install"
    IfFileExists "$INSTDIR\muteguard.exe" 0 appStoppedForInstall
    ExecWait '"$INSTDIR\muteguard.exe" --exit-all' $0
    IntCmp $0 0 appExitConfirmedForInstall
    MessageBox MB_OK|MB_ICONSTOP \
        "MuteGuard could not be closed safely (exit code $0). Close it manually and run the installer again."
    Abort
appExitConfirmedForInstall:
    Sleep 300
appStoppedForInstall:
    Delete "$INSTDIR\assets\*.*"
    RMDir "$INSTDIR\assets"
    Delete "$INSTDIR\*.md"
    SetOutPath "$INSTDIR"
    File /r /x "*.md" "${APP_DIR}/*"

    WriteUninstaller "$INSTDIR\uninstall.exe"
    CreateDirectory "$SMPROGRAMS\MuteGuard"
    CreateShortcut "$SMPROGRAMS\MuteGuard\MuteGuard.lnk" "$INSTDIR\muteguard.exe" "" "$INSTDIR\muteguard.ico" 0
    CreateShortcut "$SMPROGRAMS\MuteGuard\Uninstall MuteGuard.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\muteguard.ico" 0
    CreateShortcut "$DESKTOP\MuteGuard.lnk" "$INSTDIR\muteguard.exe" "" "$INSTDIR\muteguard.ico" 0

    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "DisplayName" "MuteGuard"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "DisplayIcon" "$INSTDIR\muteguard.exe,0"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "Publisher" "MuteGuard"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "InstallLocation" "$INSTDIR"

    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard" "EstimatedSize" "$0"

    Call EnsureWebView2
SectionEnd

Section "Uninstall"
    IfFileExists "$INSTDIR\muteguard.exe" 0 appStoppedForUninstall
    ExecWait '"$INSTDIR\muteguard.exe" --exit-all' $0
    IntCmp $0 0 appExitConfirmedForUninstall
    MessageBox MB_OK|MB_ICONSTOP \
        "MuteGuard could not be closed safely (exit code $0). Close it manually and run the uninstaller again."
    Abort
appExitConfirmedForUninstall:
    Sleep 300
appStoppedForUninstall:
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "MuteGuard"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\local.muteguard"
    Delete "$DESKTOP\MuteGuard.lnk"
    Delete "$SMPROGRAMS\MuteGuard\MuteGuard.lnk"
    Delete "$SMPROGRAMS\MuteGuard\Uninstall MuteGuard.lnk"
    RMDir "$SMPROGRAMS\MuteGuard"
    Delete "$INSTDIR\assets\*.*"
    RMDir "$INSTDIR\assets"
    Delete "$INSTDIR\*.*"
    RMDir "$INSTDIR"
SectionEnd
