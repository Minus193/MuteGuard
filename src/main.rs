#![windows_subsystem = "windows"]

use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    ffi::c_void,
    fs,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
    ptr::{null, null_mut},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, Result};
use dioxus::desktop::{
    Config as DesktopConfig, LogicalSize, WindowBuilder,
    tao::{dpi::PhysicalPosition, platform::windows::WindowBuilderExtWindows},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use windows::{
    Win32::{
        Foundation::{
            BOOL, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, GetLastError,
            HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
        },
        Graphics::{
            Dwm::{
                DWM_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
                DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE, DwmSetWindowAttribute,
            },
            Gdi::{
                CreateCompatibleDC, DEFAULT_CHARSET, DeleteDC, EnumDisplayMonitors,
                EnumFontFamiliesExW, GetMonitorInfoW, HDC, HMONITOR, LOGFONTW, MONITORINFO,
                MONITORINFOEXW, TEXTMETRICW,
            },
        },
        Media::Audio::{
            AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole,
            Endpoints::{IAudioEndpointVolume, IAudioEndpointVolumeCallback},
            IMMDevice, IMMDeviceEnumerator, IMMNotificationClient, MMDeviceEnumerator, eCapture,
            eCommunications, eConsole,
        },
        Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
        System::{
            Com::{
                CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree,
            },
            LibraryLoader::GetModuleHandleW,
            Registry::{
                HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_SZ, RRF_RT_REG_DWORD,
                RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
            },
            Threading::CreateMutexW,
        },
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem,
                SetProcessDpiAwarenessContext,
            },
            Input::KeyboardAndMouse::GetAsyncKeyState,
            Shell::{
                NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIM_ADD, NIM_DELETE,
                NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CallNextHookEx, CallWindowProcW, CreateIcon, CreateIconFromResourceEx,
                CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu,
                DestroyWindow, DispatchMessageW, FindWindowW, GWL_WNDPROC, GetCursorPos,
                GetMessageW, GetSystemMetrics, GetWindowRect, HHOOK, HICON, IDC_ARROW, IsIconic,
                KBDLLHOOKSTRUCT, KillTimer, LR_DEFAULTSIZE, LoadCursorW, MB_ICONERROR, MB_OK,
                MENU_ITEM_FLAGS, MSG, MSLLHOOKSTRUCT, MessageBoxW, PostMessageW, PostQuitMessage,
                RegisterClassW, RegisterWindowMessageW, SC_KEYMENU, SM_CXSCREEN, SM_CYSCREEN,
                SMTO_ABORTIFHUNG, SW_RESTORE, SW_SHOW, SendMessageTimeoutW, SetForegroundWindow,
                SetTimer, SetWindowLongPtrW, SetWindowsHookExW, ShowWindow, TPM_BOTTOMALIGN,
                TPM_LEFTALIGN, TPM_RETURNCMD, TrackPopupMenu, TranslateMessage,
                UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOW_EX_STYLE, WM_APP,
                WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED,
                WM_DWMCOMPOSITIONCHANGED, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
                WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETTINGCHANGE,
                WM_SYSCOMMAND, WM_THEMECHANGED, WM_TIMER, WM_WINDOWPOSCHANGED, WM_XBUTTONDOWN,
                WM_XBUTTONUP, WNDCLASSW, WNDPROC, WS_OVERLAPPED,
            },
        },
    },
    core::{PCWSTR, w},
};

mod gui;
mod native_overlay;
pub(crate) mod overlay_icons;

include!("main_parts/constants.rs");
include!("main_parts/models.rs");
include!("main_parts/globals.rs");
include!("main_parts/app_boot.rs");
include!("main_parts/tray.rs");
include!("main_parts/hotkeys_mute.rs");
include!("main_parts/overlay_runtime.rs");
include!("main_parts/audio.rs");
include!("main_parts/runtime_config.rs");
include!("main_parts/input_utils.rs");
