use std::{
    collections::HashMap,
    ffi::c_void,
    mem::size_of,
    ptr::null_mut,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicIsize, Ordering},
    },
};

use anyhow::{Context, Result};
use resvg::{tiny_skia, usvg};
use windows::{
    Win32::{
        Foundation::{BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, BeginPaint,
            CLIP_DEFAULT_PRECIS, CreateCompatibleDC, CreateDIBSection, CreateFontW,
            DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DeleteDC, DeleteObject, DrawTextW,
            EndPaint, EnumDisplayMonitors, FF_DONTCARE, FW_MEDIUM, GetMonitorInfoW,
            GetTextExtentPoint32W, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
            NONANTIALIASED_QUALITY, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SelectObject, SetBkMode,
            SetTextColor, TRANSPARENT,
        },
        UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_EXSTYLE, GetWindowLongW,
            HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, RegisterClassW, SW_HIDE,
            SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
            SWP_SHOWWINDOW, SetTimer, SetWindowLongW, SetWindowPos, ShowWindow, ULW_ALPHA,
            UpdateLayeredWindow, WM_ERASEBKGND, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::{PCWSTR, w},
};

const CLASS_NAME: PCWSTR = w!("MuteGuardOverlay");
const FADE_DURATION_MS: u32 = 300;
const FADE_STEPS: u32 = 18;
const CONTENT_TRANSITION_MS: u32 = 300;
const CONTENT_TRANSITION_STEPS: u32 = 18;
const ID_CONTENT_TRANSITION_TIMER: usize = 30;
const ID_WINDOW_FADE_TIMER: usize = 31;
const ICON_MASK_CACHE_MAX_ENTRIES: usize = 12;
const OVERLAY_EDGE_MARGIN: i32 = 10;
const OVERLAY_SIZE_FACTOR: f64 = 0.85;
const TEXT_MASK_SUPERSAMPLE: i32 = 3;
const ROUNDED_RECT_SUPERSAMPLE: usize = 8;
const DARK_BACKGROUND_RGB: (u8, u8, u8) = (19, 19, 19);
static OVERLAYS: LazyLock<Mutex<Vec<NativeOverlay>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static OVERLAY_INSTANCE: AtomicIsize = AtomicIsize::new(0);
type IconMaskKey = (String, bool, u32);
type IconMaskCache = HashMap<IconMaskKey, Vec<u8>>;
static ICON_MASK_CACHE: LazyLock<Mutex<IconMaskCache>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
struct OverlayMetrics {
    padding: i32,
    right_padding: i32,
    gap: i32,
    icon_size: i32,
    icon_font_size: i32,
    text_font_size: i32,
    text_y_offset: i32,
}

struct NativeOverlay {
    hwnd: HWND,
    muted: bool,
    transition_from_muted: Option<bool>,
    transition_progress: f64,
    transition_step: u32,
    transition_start_width: i32,
    transition_target_width: i32,
    window_alpha: f64,
    fade_from_alpha: f64,
    fade_to_alpha: f64,
    fade_step: u32,
    fade_hide_after: bool,
    settings: crate::OverlayConfig,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    visible: bool,
    monitor: MonitorRect,
    render_scale: f64,
    system_accent: (u8, u8, u8),
}

unsafe impl Send for NativeOverlay {}

pub fn init(instance: HINSTANCE, muted: bool, settings: &crate::OverlayConfig) -> Result<()> {
    let mut overlays = OVERLAYS.lock().unwrap();
    if !overlays.is_empty() {
        return Ok(());
    }

    unsafe {
        let class = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            lpfnWndProc: Some(overlay_wnd_proc),
            ..Default::default()
        };
        RegisterClassW(&class);
    }
    OVERLAY_INSTANCE.store(instance.0 as isize, Ordering::Relaxed);

    for display in selected_display_ids(settings) {
        overlays.push(create_native_overlay(instance, muted, settings, &display)?);
    }
    Ok(())
}

fn create_native_overlay(
    instance: HINSTANCE,
    muted: bool,
    settings: &crate::OverlayConfig,
    display: &str,
) -> Result<NativeOverlay> {
    let ex_style =
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            CLASS_NAME,
            w!("MuteGuard overlay"),
            WS_POPUP,
            100,
            100,
            48,
            48,
            None,
            None,
            instance,
            None,
        )
    }
    .context("create overlay window")?;

    let mut display_settings = settings.clone();
    display_settings.display = display.to_string();
    let mut native = NativeOverlay {
        hwnd,
        muted,
        transition_from_muted: None,
        transition_progress: 1.0,
        transition_step: 0,
        transition_start_width: 48,
        transition_target_width: 48,
        window_alpha: 1.0,
        fade_from_alpha: 1.0,
        fade_to_alpha: 1.0,
        fade_step: 0,
        fade_hide_after: false,
        settings: display_settings,
        width: 48,
        height: 48,
        x: 100,
        y: 100,
        visible: false,
        monitor: MonitorRect::default(),
        render_scale: 1.0,
        system_accent: crate::WindowsAccent::load().accent,
    };
    native.refresh_layout_context();
    native.apply_layout();
    native.set_click_through(true);
    Ok(native)
}

pub fn update(muted: bool, settings: &crate::OverlayConfig) {
    let desired_displays = selected_display_ids(settings);
    let mut overlays = OVERLAYS.lock().unwrap();

    let mut index = 0;
    while index < overlays.len() {
        if desired_displays.contains(&overlays[index].settings.display) {
            index += 1;
        } else {
            let removed = overlays.remove(index);
            unsafe {
                let _ = DestroyWindow(removed.hwnd);
            }
        }
    }

    let instance = HINSTANCE(OVERLAY_INSTANCE.load(Ordering::Relaxed) as *mut c_void);
    for display in &desired_displays {
        if !overlays
            .iter()
            .any(|overlay| overlay.settings.display == *display)
            && let Ok(overlay) = create_native_overlay(instance, muted, settings, display)
        {
            overlays.push(overlay);
        }
    }

    for overlay in overlays.iter_mut() {
        let mut display_settings = settings.clone();
        display_settings
            .display
            .clone_from(&overlay.settings.display);
        let next_muted = displayed_mute_state(muted, &display_settings);
        let previous_muted = overlay.muted;
        overlay.settings = display_settings;
        overlay.refresh_layout_context();
        if previous_muted != next_muted {
            overlay.start_content_transition(previous_muted, next_muted);
        } else {
            overlay.muted = next_muted;
        }
        overlay.apply_layout();
        overlay.repaint();
    }
}

fn selected_display_ids(settings: &crate::OverlayConfig) -> Vec<String> {
    let candidates = if settings.displays.is_empty() {
        vec![settings.display.clone()]
    } else {
        settings.displays.clone()
    };
    let configured = candidates
        .into_iter()
        .filter(|display| !display.trim().is_empty())
        .fold(Vec::<String>::new(), |mut displays, display| {
            if !displays.contains(&display) {
                displays.push(display);
            }
            displays
        });
    let available = monitor_rects()
        .into_iter()
        .map(|monitor| (monitor.id, monitor.primary))
        .collect::<Vec<_>>();
    select_available_display_ids(&configured, &available)
}

fn select_available_display_ids(
    configured: &[String],
    available: &[(String, bool)],
) -> Vec<String> {
    let primary_id = available
        .iter()
        .find(|(_, primary)| *primary)
        .map(|(id, _)| id.as_str());
    let mut physical_ids = Vec::<String>::new();
    let mut selected = Vec::<String>::new();

    for configured_id in configured {
        let resolved = if configured_id == crate::OVERLAY_DISPLAY_PRIMARY {
            primary_id.map(|id| (id, crate::OVERLAY_DISPLAY_PRIMARY))
        } else {
            available
                .iter()
                .find(|(id, _)| id == configured_id)
                .map(|(id, primary)| {
                    (
                        id.as_str(),
                        if *primary {
                            crate::OVERLAY_DISPLAY_PRIMARY
                        } else {
                            configured_id.as_str()
                        },
                    )
                })
        };
        let Some((physical_id, selected_id)) = resolved else {
            continue;
        };
        if physical_ids.iter().any(|id| id == physical_id) {
            continue;
        }
        physical_ids.push(physical_id.to_string());
        selected.push(selected_id.to_string());
    }

    if selected.is_empty() {
        selected.push(crate::OVERLAY_DISPLAY_PRIMARY.to_string());
    }
    selected
}

fn displayed_mute_state(muted: bool, settings: &crate::OverlayConfig) -> bool {
    match settings.visibility.as_str() {
        "WhenMuted" => true,
        "WhenUnmuted" => false,
        _ => muted,
    }
}

pub fn show() {
    for overlay in OVERLAYS.lock().unwrap().iter_mut() {
        overlay.show();
    }
}

pub fn hide() {
    for overlay in OVERLAYS.lock().unwrap().iter_mut() {
        overlay.hide();
    }
}

pub fn refresh_system_theme() {
    for overlay in OVERLAYS.lock().unwrap().iter_mut() {
        overlay.system_accent = crate::WindowsAccent::load().accent;
        overlay.repaint();
    }
}

pub fn destroy() {
    for overlay in OVERLAYS.lock().unwrap().drain(..) {
        unsafe {
            let _ = DestroyWindow(overlay.hwnd);
        }
    }
}

impl NativeOverlay {
    fn show(&mut self) {
        unsafe {
            if self.visible {
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    self.x,
                    self.y,
                    self.width,
                    self.height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                return;
            }

            self.visible = true;
            self.start_window_fade(0.0, 1.0, false);
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                self.x,
                self.y,
                self.width,
                self.height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            self.repaint();
        }
    }

    fn hide(&mut self) {
        unsafe {
            if !self.visible {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
                return;
            }

            self.visible = false;
            self.start_window_fade(self.window_alpha, 0.0, true);
        }
    }

    fn apply_layout(&mut self) {
        let target_width = self.target_width_for(self.muted);
        if self.transition_from_muted.is_some() {
            self.transition_target_width = target_width;
            self.width = lerp_i32(
                self.transition_start_width,
                self.transition_target_width,
                width_transition_progress(self.transition_progress),
            );
        } else {
            self.width = target_width;
        }

        self.x = self.saved_x();
        self.y = self.saved_y();

        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                self.x,
                self.y,
                self.width,
                self.height,
                SWP_NOACTIVATE,
            );
        }
    }

    fn target_width_for(&mut self, muted: bool) -> i32 {
        let scale = self.render_scale;
        if self.settings.variant == "Dot" {
            self.height = (24.0 * scale).round().max(4.0) as i32;
            return self.height;
        }

        self.height = (48.0 * scale).round() as i32;
        let has_icon = overlay_has_icon(&self.settings);
        let has_text = overlay_has_text(&self.settings);
        if !has_text {
            return self.height;
        }

        let metrics = overlay_metrics(self.height);
        let label = overlay_label(&self.settings, muted);
        let text_width = measure_text_width(&self.settings, label, metrics.text_font_size);
        let icon_width = if has_icon { metrics.icon_size } else { 0 };
        let left_padding = if has_icon {
            metrics.padding
        } else {
            metrics.right_padding
        };
        let right_padding = metrics.right_padding;
        let text_gap = if has_icon && text_width > 0 {
            metrics.gap
        } else {
            0
        };
        (left_padding + icon_width + text_gap + text_width + right_padding).max(self.height)
    }

    fn start_content_transition(&mut self, from_muted: bool, to_muted: bool) {
        self.transition_from_muted = Some(from_muted);
        self.transition_progress = 0.0;
        self.transition_step = 0;
        self.transition_start_width = self.width.max(1);
        self.muted = to_muted;
        self.transition_target_width = self.target_width_for(to_muted);
        unsafe {
            let _ = KillTimer(self.hwnd, ID_CONTENT_TRANSITION_TIMER);
            let _ = SetTimer(
                self.hwnd,
                ID_CONTENT_TRANSITION_TIMER,
                (CONTENT_TRANSITION_MS / CONTENT_TRANSITION_STEPS).max(1),
                None,
            );
        }
    }

    fn process_content_transition(&mut self) {
        if self.transition_from_muted.is_none() {
            unsafe {
                let _ = KillTimer(self.hwnd, ID_CONTENT_TRANSITION_TIMER);
            }
            return;
        }

        self.transition_step += 1;
        let progress =
            (self.transition_step as f64 / CONTENT_TRANSITION_STEPS as f64).clamp(0.0, 1.0);
        self.transition_progress = progress;
        self.apply_layout();
        self.repaint();

        if self.transition_step >= CONTENT_TRANSITION_STEPS {
            self.transition_from_muted = None;
            self.transition_progress = 1.0;
            self.transition_step = 0;
            self.transition_start_width = self.width;
            self.transition_target_width = self.width;
            self.apply_layout();
            self.repaint();
            unsafe {
                let _ = KillTimer(self.hwnd, ID_CONTENT_TRANSITION_TIMER);
            }
        }
    }

    fn start_window_fade(&mut self, from: f64, to: f64, hide_after: bool) {
        self.window_alpha = from.clamp(0.0, 1.0);
        self.fade_from_alpha = self.window_alpha;
        self.fade_to_alpha = to.clamp(0.0, 1.0);
        self.fade_step = 0;
        self.fade_hide_after = hide_after;
        unsafe {
            let _ = KillTimer(self.hwnd, ID_WINDOW_FADE_TIMER);
            let _ = SetTimer(
                self.hwnd,
                ID_WINDOW_FADE_TIMER,
                (FADE_DURATION_MS / FADE_STEPS).max(1),
                None,
            );
        }
    }

    fn process_window_fade(&mut self) {
        self.fade_step += 1;
        let progress = (self.fade_step as f64 / FADE_STEPS as f64).clamp(0.0, 1.0);
        self.window_alpha = self.fade_from_alpha
            + (self.fade_to_alpha - self.fade_from_alpha) * ease_in_out(progress);
        self.repaint();

        if self.fade_step >= FADE_STEPS {
            self.window_alpha = self.fade_to_alpha;
            self.repaint();
            unsafe {
                let _ = KillTimer(self.hwnd, ID_WINDOW_FADE_TIMER);
                if self.fade_hide_after {
                    let _ = ShowWindow(self.hwnd, SW_HIDE);
                }
            }
        }
    }

    fn saved_x(&self) -> i32 {
        let rect = inset_rect(self.monitor.work_area, OVERLAY_EDGE_MARGIN);
        let screen = (rect.right - rect.left).max(self.width);
        rect.left + percent_to_axis(self.settings.position_x, screen, self.width)
    }

    fn saved_y(&self) -> i32 {
        let rect = inset_rect(self.monitor.work_area, OVERLAY_EDGE_MARGIN);
        let screen = (rect.bottom - rect.top).max(self.height);
        rect.top + percent_to_axis(self.settings.position_y, screen, self.height)
    }

    fn refresh_layout_context(&mut self) {
        self.monitor = selected_monitor(&self.settings.display);
        let dpi = monitor_dpi_scale(self.monitor.handle).unwrap_or_else(|| dpi_scale(self.hwnd));
        self.render_scale = overlay_render_scale(self.settings.scale, dpi);
    }

    fn set_click_through(&self, click_through: bool) {
        unsafe {
            let style = GetWindowLongW(self.hwnd, GWL_EXSTYLE);
            let transparent = WS_EX_TRANSPARENT.0 as i32;
            let next_style = if click_through {
                style | transparent
            } else {
                style & !transparent
            };
            let _ = SetWindowLongW(self.hwnd, GWL_EXSTYLE, next_style);
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER,
            );
        }
    }

    fn repaint(&self) {
        self.render_layered();
    }

    fn render_layered(&self) {
        if self.width <= 0 || self.height <= 0 {
            return;
        }

        unsafe {
            let screen_hdc = CreateCompatibleDC(None);
            if screen_hdc.0.is_null() {
                return;
            }

            let mut bits: *mut c_void = null_mut();
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: self.width,
                    biHeight: -self.height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let bitmap =
                match CreateDIBSection(screen_hdc, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                    Ok(bitmap) => bitmap,
                    Err(_) => {
                        let _ = DeleteDC(screen_hdc);
                        return;
                    }
                };

            let old_bitmap = SelectObject(screen_hdc, bitmap);
            clear_argb(bits, self.width, self.height);
            self.paint_surface(bits);
            self.update_layered(screen_hdc);
            let _ = SelectObject(screen_hdc, old_bitmap);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(screen_hdc);
        }
    }

    fn paint_surface(&self, bits: *mut c_void) {
        if self.settings.variant == "Dot" {
            self.paint_dot_surface(bits);
            return;
        }

        let dark_background = self.settings.background_style != "Light";
        let background_opacity = if self.settings.background_style == "Transparent" {
            0
        } else {
            self.settings.background_opacity
        };
        let background_rgb = if dark_background {
            DARK_BACKGROUND_RGB
        } else {
            (255, 255, 255)
        };
        let border_rgb =
            crate::parse_hex_color(&self.settings.border_color).unwrap_or((50, 52, 65));
        paint_antialiased_rounded_rect(
            bits,
            self.width,
            self.height,
            self.scaled_corner_radius(),
            background_rgb,
            (background_opacity.min(100) as f64 / 100.0) * self.window_alpha,
            self.settings
                .show_border
                .then_some((border_rgb, self.window_alpha)),
        );
        self.paint_content(bits, dark_background);
    }

    fn paint_dot_surface(&self, bits: *mut c_void) {
        let dot_color = self.transition_from_muted.map_or_else(
            || state_accent(self.muted),
            |from_muted| {
                transition_color(
                    state_accent(from_muted),
                    state_accent(self.muted),
                    content_in_opacity(self.transition_progress),
                )
            },
        );
        let content_opacity = self.settings.content_opacity.clamp(20, 100);
        let accent = blend_rgb((0, 0, 0), dot_color, f64::from(content_opacity) / 100.0);
        paint_antialiased_rounded_rect(
            bits,
            self.width,
            self.height,
            self.scaled_corner_radius(),
            accent,
            (f64::from(content_opacity) / 100.0) * self.window_alpha,
            None,
        );
    }

    fn paint_content(&self, bits: *mut c_void, dark_background: bool) {
        let metrics = overlay_metrics(self.height);
        if let Some(from_muted) = self.transition_from_muted {
            self.compose_content(
                bits,
                from_muted,
                content_out_opacity(self.transition_progress),
                dark_background,
                &metrics,
            );
            self.compose_content(
                bits,
                self.muted,
                content_in_opacity(self.transition_progress),
                dark_background,
                &metrics,
            );
        } else {
            self.compose_content(bits, self.muted, 1.0, dark_background, &metrics);
        }
    }

    fn scaled_corner_radius(&self) -> i32 {
        (f64::from(self.settings.border_radius.min(24)) * self.render_scale).round() as i32
    }

    fn update_layered(&self, hdc: HDC) {
        unsafe {
            let dst = POINT {
                x: self.x,
                y: self.y,
            };
            let size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: 0,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                self.hwnd,
                None,
                Some(&dst),
                Some(&size),
                hdc,
                Some(&src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }

    fn compose_content(
        &self,
        target_bits: *mut c_void,
        muted: bool,
        opacity_factor: f64,
        dark_background: bool,
        metrics: &OverlayMetrics,
    ) {
        let opacity = (self.settings.content_opacity.clamp(20, 100) as f64 / 100.0)
            * opacity_factor.clamp(0.0, 1.0)
            * self.window_alpha;
        if opacity <= 0.0 {
            return;
        }

        let has_icon = overlay_has_icon(&self.settings);
        let has_text = overlay_has_text(&self.settings);
        if has_icon {
            self.compose_icon(
                target_bits,
                muted,
                opacity,
                dark_background,
                has_text,
                metrics,
            );
        }
        if has_text {
            self.compose_label(
                target_bits,
                muted,
                opacity,
                dark_background,
                has_icon,
                metrics,
            );
        }
    }

    fn content_icon_color(&self, muted: bool, dark_background: bool) -> (u8, u8, u8) {
        match self.settings.icon_style.as_str() {
            "Monochrome" => {
                if dark_background {
                    (255, 255, 255)
                } else {
                    (0, 0, 0)
                }
            }
            "SystemColor" => self.system_accent,
            "Custom" => {
                crate::parse_hex_color(&self.settings.icon_color).unwrap_or(self.system_accent)
            }
            _ => state_accent(muted),
        }
    }

    fn compose_icon(
        &self,
        target_bits: *mut c_void,
        muted: bool,
        opacity: f64,
        dark_background: bool,
        has_text: bool,
        metrics: &OverlayMetrics,
    ) {
        let icon_color = self.content_icon_color(muted, dark_background);
        let icon_left = if has_text {
            metrics.padding
        } else {
            (self.width - metrics.icon_size) / 2
        };
        let icon_top = (self.height - metrics.icon_size) / 2;
        if let Some(mask) = overlay_icon_mask(
            &self.settings.icon_pair,
            muted,
            metrics.icon_size.max(1) as u32,
        ) {
            composite_masked_subrect(
                target_bits,
                self.width,
                self.height,
                MaskSubrect {
                    mask: &mask,
                    width: metrics.icon_size.max(1),
                    height: metrics.icon_size.max(1),
                    offset_x: icon_left.max(0),
                    offset_y: icon_top.max(0),
                    color: icon_color,
                    opacity,
                },
            );
            return;
        }

        let Some(mask) = render_text_mask(self.width, self.height, |hdc, supersample| unsafe {
            let icon_face = crate::wide("Segoe Fluent Icons");
            let icon_font = CreateFontW(
                metrics.icon_font_size * supersample,
                0,
                0,
                0,
                FW_MEDIUM.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                NONANTIALIASED_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(icon_face.as_ptr()),
            );
            let old_font = SelectObject(hdc, icon_font);
            let glyph = if muted { "\u{F781}" } else { "\u{E720}" };
            let mut icon_text: Vec<u16> = glyph.encode_utf16().collect();
            let mut icon_rect = RECT {
                left: if has_text {
                    metrics.padding * supersample
                } else {
                    0
                },
                top: 0,
                right: if has_text {
                    (metrics.padding + metrics.icon_size) * supersample
                } else {
                    self.width * supersample
                },
                bottom: self.height * supersample,
            };
            DrawTextW(
                hdc,
                &mut icon_text,
                &mut icon_rect,
                windows::Win32::Graphics::Gdi::DT_CENTER
                    | windows::Win32::Graphics::Gdi::DT_VCENTER
                    | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
            );
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(icon_font);
        }) else {
            return;
        };
        composite_masked_color(
            target_bits,
            self.width,
            self.height,
            &mask,
            icon_color,
            opacity,
        );
    }

    fn compose_label(
        &self,
        target_bits: *mut c_void,
        muted: bool,
        opacity: f64,
        dark_background: bool,
        has_icon: bool,
        metrics: &OverlayMetrics,
    ) {
        let label = overlay_label(&self.settings, muted);
        if label.is_empty() {
            return;
        }

        let text_color = if dark_background {
            (245, 245, 245)
        } else {
            (18, 18, 18)
        };
        let Some(mask) = render_text_mask(self.width, self.height, |hdc, supersample| unsafe {
            let text_face = overlay_text_font_face(&self.settings);
            let text_font = CreateFontW(
                metrics.text_font_size * supersample,
                0,
                0,
                0,
                overlay_text_font_weight(&self.settings),
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                NONANTIALIASED_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(text_face.as_ptr()),
            );
            let old_font = SelectObject(hdc, text_font);
            let mut label: Vec<u16> = label.encode_utf16().collect();
            let text_left = if has_icon {
                metrics.padding + metrics.icon_size + metrics.gap
            } else {
                metrics.right_padding
            };
            let mut text_rect = RECT {
                left: text_left * supersample,
                top: metrics.text_y_offset * supersample,
                right: (self.width - metrics.right_padding) * supersample,
                bottom: (self.height + metrics.text_y_offset) * supersample,
            };
            DrawTextW(
                hdc,
                &mut label,
                &mut text_rect,
                windows::Win32::Graphics::Gdi::DT_VCENTER
                    | windows::Win32::Graphics::Gdi::DT_SINGLELINE
                    | windows::Win32::Graphics::Gdi::DT_NOPREFIX,
            );
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(text_font);
        }) else {
            return;
        };
        composite_masked_color(
            target_bits,
            self.width,
            self.height,
            &mask,
            text_color,
            opacity,
        );
    }
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            unsafe {
                let _ = BeginPaint(hwnd, &mut paint);
            }
            if let Some(overlay) = OVERLAYS
                .lock()
                .unwrap()
                .iter()
                .find(|overlay| overlay.hwnd == hwnd)
            {
                overlay.repaint();
            }
            unsafe {
                let _ = EndPaint(hwnd, &paint);
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == ID_CONTENT_TRANSITION_TIMER => {
            if let Some(overlay) = OVERLAYS
                .lock()
                .unwrap()
                .iter_mut()
                .find(|overlay| overlay.hwnd == hwnd)
            {
                overlay.process_content_transition();
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == ID_WINDOW_FADE_TIMER => {
            if let Some(overlay) = OVERLAYS
                .lock()
                .unwrap()
                .iter_mut()
                .find(|overlay| overlay.hwnd == hwnd)
            {
                overlay.process_window_fade();
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn percent_to_axis(percent: f64, screen: i32, size: i32) -> i32 {
    let available = (screen - size).max(0) as f64;
    (available * percent.clamp(0.0, 100.0) / 100.0).round() as i32
}

#[cfg(test)]
fn axis_to_percent(position: i32, screen: i32, size: i32) -> f64 {
    let available = (screen - size).max(1) as f64;
    (position as f64 * 100.0 / available).clamp(0.0, 100.0)
}

fn inset_rect(rect: RECT, margin: i32) -> RECT {
    let horizontal = margin.max(0).min((rect.right - rect.left).max(0) / 2);
    let vertical = margin.max(0).min((rect.bottom - rect.top).max(0) / 2);
    RECT {
        left: rect.left + horizontal,
        top: rect.top + vertical,
        right: rect.right - horizontal,
        bottom: rect.bottom - vertical,
    }
}

fn selected_monitor(display: &str) -> MonitorRect {
    let monitors = monitor_rects();
    let primary = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first());

    if display == crate::OVERLAY_DISPLAY_PRIMARY || display.is_empty() {
        return primary.cloned().unwrap_or_default();
    }

    monitors
        .iter()
        .find(|monitor| monitor.id == display)
        .or_else(|| {
            display
                .strip_prefix("Monitor")
                .and_then(|index| index.parse::<usize>().ok())
                .and_then(|index| monitors.get(index.saturating_sub(1)))
        })
        .or(primary)
        .cloned()
        .unwrap_or_default()
}

fn monitor_rects() -> Vec<MonitorRect> {
    let mut monitors = Vec::<MonitorRect>::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(collect_monitor_rect),
            LPARAM(&mut monitors as *mut _ as isize),
        );
    }
    monitors
}

#[derive(Clone, Default)]
struct MonitorRect {
    handle: HMONITOR,
    id: String,
    work_area: RECT,
    primary: bool,
}

unsafe extern "system" fn collect_monitor_rect(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(data.0 as *mut Vec<MonitorRect>) };
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool() {
        monitors.push(MonitorRect {
            handle: monitor,
            id: wide_device_name(&info.szDevice),
            work_area: info.monitorInfo.rcWork,
            primary: (info.monitorInfo.dwFlags & 1) != 0,
        });
    }
    true.into()
}

fn wide_device_name(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

fn overlay_metrics(height: i32) -> OverlayMetrics {
    let scale = height as f64 / 48.0;
    OverlayMetrics {
        padding: (8.0 * scale).round().max(3.0) as i32,
        right_padding: (12.0 * scale).round().max(4.0) as i32,
        gap: (8.0 * scale).round().max(3.0) as i32,
        icon_size: (34.0 * scale).round().max(12.0) as i32,
        icon_font_size: -((height as f64 * 0.58).round() as i32),
        text_font_size: -((height as f64 * 0.33).round() as i32),
        text_y_offset: (-((1.5 * scale).round() as i32)).min(-1),
    }
}

fn overlay_label(settings: &crate::OverlayConfig, muted: bool) -> &str {
    if muted {
        &settings.muted_label
    } else {
        &settings.unmuted_label
    }
}

fn overlay_has_icon(settings: &crate::OverlayConfig) -> bool {
    matches!(settings.variant.as_str(), "MicIcon" | "IconText")
}

fn overlay_has_text(settings: &crate::OverlayConfig) -> bool {
    matches!(settings.variant.as_str(), "IconText" | "Text")
        || (settings.variant == "MicIcon" && settings.show_text)
}

fn measure_text_width(settings: &crate::OverlayConfig, text: &str, font_size: i32) -> i32 {
    if text.is_empty() {
        return 0;
    }

    unsafe {
        let hdc = CreateCompatibleDC(None);
        if hdc.0.is_null() {
            return fallback_text_width(text, font_size);
        }

        let text_face = overlay_text_font_face(settings);
        let font = CreateFontW(
            font_size * TEXT_MASK_SUPERSAMPLE,
            0,
            0,
            0,
            overlay_text_font_weight(settings),
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            NONANTIALIASED_QUALITY.0 as u32,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            PCWSTR(text_face.as_ptr()),
        );
        let old_font = SelectObject(hdc, font);
        let text_utf16: Vec<u16> = text.encode_utf16().collect();
        let mut size = SIZE::default();
        let measured = GetTextExtentPoint32W(hdc, &text_utf16, &mut size).as_bool();
        let _ = SelectObject(hdc, old_font);
        let _ = DeleteObject(font);
        let _ = DeleteDC(hdc);

        if measured {
            ((size.cx + TEXT_MASK_SUPERSAMPLE - 1) / TEXT_MASK_SUPERSAMPLE).max(1)
        } else {
            fallback_text_width(text, font_size)
        }
    }
}

fn fallback_text_width(text: &str, font_size: i32) -> i32 {
    let px = font_size.abs().max(1) as f64;
    (text.chars().count() as f64 * px * 0.56).round() as i32
}

fn overlay_text_font_face(settings: &crate::OverlayConfig) -> Vec<u16> {
    let family = settings.text_font.trim();
    if family.is_empty() {
        crate::wide("Segoe UI")
    } else {
        crate::wide(family)
    }
}

fn overlay_text_font_weight(settings: &crate::OverlayConfig) -> i32 {
    i32::from(settings.text_font_weight.clamp(100, 900))
}

fn render_text_mask(width: i32, height: i32, draw: impl FnOnce(HDC, i32)) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 {
        return None;
    }

    let raster_width = width.checked_mul(TEXT_MASK_SUPERSAMPLE)?;
    let raster_height = height.checked_mul(TEXT_MASK_SUPERSAMPLE)?;

    unsafe {
        let hdc = CreateCompatibleDC(None);
        if hdc.0.is_null() {
            return None;
        }

        let mut bits: *mut c_void = null_mut();
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: raster_width,
                biHeight: -raster_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let bitmap = match CreateDIBSection(hdc, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) => bitmap,
            Err(_) => {
                let _ = DeleteDC(hdc);
                return None;
            }
        };

        let old_bitmap = SelectObject(hdc, bitmap);
        clear_argb_to(bits, raster_width, raster_height, 0);
        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, colorref(255, 255, 255));
        draw(hdc, TEXT_MASK_SUPERSAMPLE);

        let mask = if bits.is_null() {
            None
        } else {
            let pixels = std::slice::from_raw_parts(
                bits as *const u32,
                (raster_width * raster_height) as usize,
            );
            let sample_count = (TEXT_MASK_SUPERSAMPLE * TEXT_MASK_SUPERSAMPLE) as u32;
            let mut downsampled = vec![0; (width * height) as usize];
            for y in 0..height {
                for x in 0..width {
                    let mut coverage_sum = 0_u32;
                    for sample_y in 0..TEXT_MASK_SUPERSAMPLE {
                        for sample_x in 0..TEXT_MASK_SUPERSAMPLE {
                            let source_x = x * TEXT_MASK_SUPERSAMPLE + sample_x;
                            let source_y = y * TEXT_MASK_SUPERSAMPLE + sample_y;
                            let source_index = (source_y * raster_width + source_x) as usize;
                            let [blue, green, red, _] = pixels[source_index].to_le_bytes();
                            coverage_sum += u32::from(red.max(green).max(blue));
                        }
                    }
                    downsampled[(y * width + x) as usize] =
                        ((coverage_sum + sample_count / 2) / sample_count) as u8;
                }
            }
            Some(downsampled)
        };

        let _ = SelectObject(hdc, old_bitmap);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(hdc);
        mask
    }
}

fn clear_argb(bits: *mut c_void, width: i32, height: i32) {
    clear_argb_to(bits, width, height, 0);
}

fn clear_argb_to(bits: *mut c_void, width: i32, height: i32, value: u32) {
    if bits.is_null() || width <= 0 || height <= 0 {
        return;
    }

    unsafe {
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
        pixels.fill(value);
    }
}

fn composite_masked_color(
    target_bits: *mut c_void,
    width: i32,
    height: i32,
    mask: &[u8],
    color: (u8, u8, u8),
    opacity: f64,
) {
    if target_bits.is_null() || width <= 0 || height <= 0 {
        return;
    }

    let pixel_count = (width * height) as usize;
    if mask.len() < pixel_count {
        return;
    }

    let opacity = opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return;
    }

    unsafe {
        let target = std::slice::from_raw_parts_mut(target_bits as *mut u32, pixel_count);
        for (dst, coverage) in target.iter_mut().zip(mask.iter()) {
            let src_alpha = (*coverage as f64 / 255.0) * opacity;
            if src_alpha <= 0.0 {
                continue;
            }

            let [dst_b, dst_g, dst_r, dst_a] = dst.to_le_bytes();
            let inv_alpha = 1.0 - src_alpha;
            let out_a = src_alpha + (dst_a as f64 / 255.0) * inv_alpha;
            let out_r = color.0 as f64 * src_alpha + dst_r as f64 * inv_alpha;
            let out_g = color.1 as f64 * src_alpha + dst_g as f64 * inv_alpha;
            let out_b = color.2 as f64 * src_alpha + dst_b as f64 * inv_alpha;

            *dst = u32::from_le_bytes([
                out_b.round().clamp(0.0, 255.0) as u8,
                out_g.round().clamp(0.0, 255.0) as u8,
                out_r.round().clamp(0.0, 255.0) as u8,
                (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
            ]);
        }
    }
}

#[derive(Clone, Copy)]
struct MaskSubrect<'a> {
    mask: &'a [u8],
    width: i32,
    height: i32,
    offset_x: i32,
    offset_y: i32,
    color: (u8, u8, u8),
    opacity: f64,
}

fn composite_masked_subrect(
    target_bits: *mut c_void,
    target_width: i32,
    target_height: i32,
    subrect: MaskSubrect<'_>,
) {
    let MaskSubrect {
        mask,
        width: mask_width,
        height: mask_height,
        offset_x,
        offset_y,
        color,
        opacity,
    } = subrect;
    if target_bits.is_null()
        || target_width <= 0
        || target_height <= 0
        || mask_width <= 0
        || mask_height <= 0
    {
        return;
    }

    let pixel_count = (mask_width * mask_height) as usize;
    if mask.len() < pixel_count {
        return;
    }

    let opacity = opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return;
    }

    unsafe {
        let target = std::slice::from_raw_parts_mut(
            target_bits as *mut u32,
            (target_width * target_height) as usize,
        );
        for mask_y in 0..mask_height {
            let dst_y = offset_y + mask_y;
            if !(0..target_height).contains(&dst_y) {
                continue;
            }
            for mask_x in 0..mask_width {
                let dst_x = offset_x + mask_x;
                if !(0..target_width).contains(&dst_x) {
                    continue;
                }

                let mask_index = (mask_y * mask_width + mask_x) as usize;
                let coverage = mask[mask_index];
                let src_alpha = (coverage as f64 / 255.0) * opacity;
                if src_alpha <= 0.0 {
                    continue;
                }

                let dst_index = (dst_y * target_width + dst_x) as usize;
                let dst = &mut target[dst_index];
                let [dst_b, dst_g, dst_r, dst_a] = dst.to_le_bytes();
                let inv_alpha = 1.0 - src_alpha;
                let out_a = src_alpha + (dst_a as f64 / 255.0) * inv_alpha;
                let out_r = color.0 as f64 * src_alpha + dst_r as f64 * inv_alpha;
                let out_g = color.1 as f64 * src_alpha + dst_g as f64 * inv_alpha;
                let out_b = color.2 as f64 * src_alpha + dst_b as f64 * inv_alpha;

                *dst = u32::from_le_bytes([
                    out_b.round().clamp(0.0, 255.0) as u8,
                    out_g.round().clamp(0.0, 255.0) as u8,
                    out_r.round().clamp(0.0, 255.0) as u8,
                    (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
                ]);
            }
        }
    }
}

fn overlay_icon_mask(icon_pair: &str, muted: bool, size: u32) -> Option<Vec<u8>> {
    let key = (icon_pair.to_string(), muted, size);
    if let Some(mask) = ICON_MASK_CACHE.lock().unwrap().get(&key).cloned() {
        return Some(mask);
    }

    let svg = crate::overlay_icons::overlay_icon_svg(icon_pair, muted);
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let svg_size = tree.size().to_int_size();
    let scale = (size as f32 / svg_size.width() as f32).min(size as f32 / svg_size.height() as f32);
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let pixels = pixmap.take_demultiplied();
    let raw_mask = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3])
        .collect::<Vec<_>>();
    let mask = recenter_alpha_mask(&raw_mask, size as usize, size as usize);
    insert_icon_mask(&mut ICON_MASK_CACHE.lock().unwrap(), key, mask.clone());
    Some(mask)
}

fn insert_icon_mask(cache: &mut IconMaskCache, key: IconMaskKey, mask: Vec<u8>) {
    if cache.len() >= ICON_MASK_CACHE_MAX_ENTRIES && !cache.contains_key(&key) {
        cache.clear();
    }
    cache.insert(key, mask);
}

fn recenter_alpha_mask(mask: &[u8], width: usize, height: usize) -> Vec<u8> {
    if width == 0 || height == 0 || mask.len() < width * height {
        return mask.to_vec();
    }

    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut has_pixels = false;

    for y in 0..height {
        for x in 0..width {
            if mask[y * width + x] == 0 {
                continue;
            }
            has_pixels = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if !has_pixels {
        return mask.to_vec();
    }

    let bounds_width = max_x - min_x + 1;
    let bounds_height = max_y - min_y + 1;
    let target_x = width.saturating_sub(bounds_width) / 2;
    let target_y = height.saturating_sub(bounds_height) / 2;

    if min_x == target_x && min_y == target_y {
        return mask.to_vec();
    }

    let mut centered = vec![0; width * height];
    for row in 0..bounds_height {
        let src_start = (min_y + row) * width + min_x;
        let src_end = src_start + bounds_width;
        let dst_start = (target_y + row) * width + target_x;
        let dst_end = dst_start + bounds_width;
        centered[dst_start..dst_end].copy_from_slice(&mask[src_start..src_end]);
    }

    centered
}

fn paint_antialiased_rounded_rect(
    bits: *mut c_void,
    width: i32,
    height: i32,
    radius: i32,
    fill_rgb: (u8, u8, u8),
    fill_opacity: f64,
    border: Option<((u8, u8, u8), f64)>,
) {
    if bits.is_null() || width <= 0 || height <= 0 {
        return;
    }

    let fill_opacity = fill_opacity.clamp(0.0, 1.0);
    let radius = radius.max(0) as f64;
    unsafe {
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let outer_coverage = rounded_rect_coverage(width, height, radius, x, y, 0.0);
                if outer_coverage <= 0.0 {
                    continue;
                }

                let base_alpha = fill_opacity * outer_coverage;
                let mut red = f64::from(fill_rgb.0) * base_alpha;
                let mut green = f64::from(fill_rgb.1) * base_alpha;
                let mut blue = f64::from(fill_rgb.2) * base_alpha;
                let mut alpha = base_alpha;

                if let Some((border_rgb, border_opacity)) = border {
                    let inner_coverage = rounded_rect_coverage(width, height, radius, x, y, 1.0);
                    let border_coverage = (outer_coverage - inner_coverage).clamp(0.0, 1.0);
                    let border_alpha = border_opacity.clamp(0.0, 1.0) * border_coverage;
                    let keep_base = 1.0 - border_alpha;
                    red = f64::from(border_rgb.0) * border_alpha + red * keep_base;
                    green = f64::from(border_rgb.1) * border_alpha + green * keep_base;
                    blue = f64::from(border_rgb.2) * border_alpha + blue * keep_base;
                    alpha = border_alpha + alpha * keep_base;
                }

                pixels[(y * width + x) as usize] = premultiplied_pixel(red, green, blue, alpha);
            }
        }
    }
}

fn rounded_rect_coverage(
    width: i32,
    height: i32,
    radius: f64,
    pixel_x: i32,
    pixel_y: i32,
    inset: f64,
) -> f64 {
    let left = inset.max(0.0);
    let top = left;
    let right = f64::from(width) - left;
    let bottom = f64::from(height) - top;
    if right <= left || bottom <= top {
        return 0.0;
    }

    let radius = (radius - left)
        .max(0.0)
        .min((right - left) / 2.0)
        .min((bottom - top) / 2.0);
    let mut inside = 0usize;
    for sample_y in 0..ROUNDED_RECT_SUPERSAMPLE {
        let y = f64::from(pixel_y) + (sample_y as f64 + 0.5) / ROUNDED_RECT_SUPERSAMPLE as f64;
        for sample_x in 0..ROUNDED_RECT_SUPERSAMPLE {
            let x = f64::from(pixel_x) + (sample_x as f64 + 0.5) / ROUNDED_RECT_SUPERSAMPLE as f64;
            if point_inside_rounded_rect(x, y, left, top, right, bottom, radius) {
                inside += 1;
            }
        }
    }
    inside as f64 / (ROUNDED_RECT_SUPERSAMPLE * ROUNDED_RECT_SUPERSAMPLE) as f64
}

fn point_inside_rounded_rect(
    x: f64,
    y: f64,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    radius: f64,
) -> bool {
    if x < left || x >= right || y < top || y >= bottom {
        return false;
    }
    if radius <= 0.0 {
        return true;
    }

    let nearest_x = x.clamp(left + radius, right - radius);
    let nearest_y = y.clamp(top + radius, bottom - radius);
    let dx = x - nearest_x;
    let dy = y - nearest_y;
    dx * dx + dy * dy <= radius * radius
}

fn premultiplied_pixel(red: f64, green: f64, blue: f64, alpha: f64) -> u32 {
    let channel = |value: f64| value.round().clamp(0.0, 255.0) as u8;
    u32::from_le_bytes([
        channel(blue),
        channel(green),
        channel(red),
        channel(alpha.clamp(0.0, 1.0) * 255.0),
    ])
}

fn dpi_scale(hwnd: HWND) -> f64 {
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    dpi as f64 / 96.0
}

fn monitor_dpi_scale(monitor: HMONITOR) -> Option<f64> {
    if monitor.0.is_null() {
        return None;
    }

    let mut dpi_x = 96;
    let mut dpi_y = 96;
    unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }.ok()?;
    Some(dpi_x.max(dpi_y).max(96) as f64 / 96.0)
}

fn overlay_render_scale(user_percent: u32, dpi_scale: f64) -> f64 {
    user_percent.clamp(10, 400) as f64 / 100.0 * OVERLAY_SIZE_FACTOR * dpi_scale.max(1.0)
}

fn colorref(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}

fn state_accent(muted: bool) -> (u8, u8, u8) {
    if muted { (220, 53, 69) } else { (40, 167, 69) }
}

fn transition_color(from: (u8, u8, u8), to: (u8, u8, u8), progress: f64) -> (u8, u8, u8) {
    (
        lerp_u8(from.0, to.0, progress),
        lerp_u8(from.1, to.1, progress),
        lerp_u8(from.2, to.2, progress),
    )
}

fn blend_rgb(from: (u8, u8, u8), to: (u8, u8, u8), amount: f64) -> (u8, u8, u8) {
    (
        lerp_u8(from.0, to.0, amount),
        lerp_u8(from.1, to.1, amount),
        lerp_u8(from.2, to.2, amount),
    )
}

fn lerp_i32(from: i32, to: i32, progress: f64) -> i32 {
    (from as f64 + (to - from) as f64 * progress.clamp(0.0, 1.0)).round() as i32
}

fn lerp_u8(from: u8, to: u8, progress: f64) -> u8 {
    (from as f64 + (to as f64 - from as f64) * progress.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn width_transition_progress(progress: f64) -> f64 {
    ease_in_out((progress / 0.5).clamp(0.0, 1.0))
}

fn content_out_opacity(progress: f64) -> f64 {
    if progress < 0.5 {
        1.0 - ease_in_out(progress / 0.5)
    } else {
        0.0
    }
}

fn content_in_opacity(progress: f64) -> f64 {
    if progress < 0.5 {
        0.0
    } else {
        let local = (progress - 0.5) / 0.5;
        ease_in_out(local)
    }
}

fn ease_in_out(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_position_conversion_round_trips_and_clamps() {
        let axis = percent_to_axis(50.0, 1920, 48);
        assert_eq!(axis, 936);
        assert!((axis_to_percent(axis, 1920, 48) - 50.0).abs() < 0.01);
        assert_eq!(percent_to_axis(-10.0, 1920, 48), 0);
        assert_eq!(percent_to_axis(110.0, 1920, 48), 1872);
        assert_eq!(axis_to_percent(-100, 1920, 48), 0.0);
        assert_eq!(axis_to_percent(2_000, 1920, 48), 100.0);
    }

    #[test]
    fn overlay_work_area_excludes_taskbar_and_keeps_ten_pixel_margin() {
        let work_area = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let placement = inset_rect(work_area, OVERLAY_EDGE_MARGIN);
        assert_eq!(placement.left, 10);
        assert_eq!(placement.top, 10);
        assert_eq!(placement.right, 1910);
        assert_eq!(placement.bottom, 1030);

        let y = placement.top + percent_to_axis(100.0, placement.bottom - placement.top, 48);
        assert_eq!(y + 48, 1030);
    }

    #[test]
    fn rounded_overlay_edges_have_fractional_pixel_coverage() {
        assert_eq!(rounded_rect_coverage(48, 48, 6.0, 0, 0, 0.0), 0.0);
        let curved_edge = rounded_rect_coverage(48, 48, 6.0, 1, 2, 0.0);
        assert!(curved_edge > 0.0 && curved_edge < 1.0);
        assert_eq!(rounded_rect_coverage(48, 48, 6.0, 24, 24, 0.0), 1.0);
    }

    #[test]
    fn overlay_surface_starts_fully_transparent() {
        let mut pixels = vec![0x00ff00ff_u32; 16];
        clear_argb(pixels.as_mut_ptr().cast(), 4, 4);
        assert!(pixels.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn one_hundred_percent_uses_eighty_five_percent_of_the_previous_geometry() {
        assert!((overlay_render_scale(100, 1.0) - 0.85).abs() < f64::EPSILON);
        assert!((overlay_render_scale(200, 1.0) - 1.70).abs() < f64::EPSILON);
        assert!((overlay_render_scale(100, 1.5) - 1.275).abs() < f64::EPSILON);
    }

    #[test]
    fn selected_overlay_displays_are_unique_and_never_empty() {
        let available = vec![
            (r"\\.\DISPLAY1".to_string(), true),
            (r"\\.\DISPLAY2".to_string(), false),
        ];
        assert_eq!(
            select_available_display_ids(&[], &available),
            [crate::OVERLAY_DISPLAY_PRIMARY]
        );
        assert_eq!(
            select_available_display_ids(
                &[r"\\.\DISPLAY2".to_string(), r"\\.\DISPLAY2".to_string(),],
                &available,
            ),
            [r"\\.\DISPLAY2"]
        );
    }

    #[test]
    fn unavailable_or_duplicate_physical_displays_fall_back_only_once() {
        let available = vec![
            (r"\\.\DISPLAY1".to_string(), true),
            (r"\\.\DISPLAY2".to_string(), false),
        ];
        assert_eq!(
            select_available_display_ids(
                &["disconnected-a".to_string(), "disconnected-b".to_string()],
                &available,
            ),
            [crate::OVERLAY_DISPLAY_PRIMARY]
        );
        assert_eq!(
            select_available_display_ids(
                &[
                    crate::OVERLAY_DISPLAY_PRIMARY.to_string(),
                    r"\\.\DISPLAY1".to_string(),
                    r"\\.\DISPLAY2".to_string(),
                ],
                &available,
            ),
            [crate::OVERLAY_DISPLAY_PRIMARY, r"\\.\DISPLAY2"]
        );
    }

    #[test]
    fn icon_mask_cache_stays_bounded() {
        let mut cache = IconMaskCache::new();
        for size in 0..=ICON_MASK_CACHE_MAX_ENTRIES as u32 {
            insert_icon_mask(
                &mut cache,
                ("fluent".to_string(), false, size),
                vec![size as u8],
            );
            assert!(cache.len() <= ICON_MASK_CACHE_MAX_ENTRIES);
        }
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn alpha_mask_is_recentred_without_changing_its_pixels() {
        let source = vec![
            0, 0, 0, 0, //
            7, 9, 0, 0, //
            0, 0, 0, 0, //
            0, 0, 0, 0,
        ];
        let centered = recenter_alpha_mask(&source, 4, 4);
        assert_eq!(centered[5..7], [7, 9]);
        assert_eq!(centered.iter().filter(|pixel| **pixel != 0).count(), 2);
    }
}
