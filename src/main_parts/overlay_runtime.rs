fn apply_overlay_visibility() {
    let (muted, audio_available, preview_active, overlay) = {
        let state = STATE.lock().unwrap();
        (
            state.muted,
            state.audio_available,
            state.overlay_preview_active,
            state.overlay.clone(),
        )
    };

    if preview_active {
        native_overlay::update(muted, &overlay);
        native_overlay::show();
        return;
    }

    if !overlay.enabled || !audio_available {
        native_overlay::hide();
        return;
    }

    let should_show = match overlay.visibility.as_str() {
        "Always" => true,
        "WhenMuted" => muted,
        "WhenUnmuted" => !muted,
        "AfterToggle" => false,
        _ => muted,
    };

    if should_show {
        native_overlay::update(muted, &overlay);
        native_overlay::show();
    } else {
        native_overlay::hide();
    }
}

fn show_overlay_temporarily(duration_ms: u32) {
    let (hwnd, muted, audio_available, overlay) = {
        let state = STATE.lock().unwrap();
        (
            state.hwnd,
            state.muted,
            state.audio_available,
            state.overlay.clone(),
        )
    };
    if !audio_available {
        return;
    }
    display_overlay_temporarily(hwnd, muted, &overlay, duration_ms);
}

fn set_overlay_preview(enabled: bool) {
    let (hwnd, muted, overlay) = {
        let mut state = STATE.lock().unwrap();
        state.overlay_preview_active = enabled;
        (state.hwnd, state.muted, state.overlay.clone())
    };

    unsafe {
        let _ = KillTimer(hwnd, ID_OVERLAY_PREVIEW_LEASE_TIMER);
    }
    if enabled {
        native_overlay::update(muted, &overlay);
        native_overlay::show();
        unsafe {
            let _ = SetTimer(
                hwnd,
                ID_OVERLAY_PREVIEW_LEASE_TIMER,
                OVERLAY_PREVIEW_LEASE_MS,
                None,
            );
        }
    } else {
        apply_overlay_visibility();
    }
}

fn display_overlay_temporarily(
    hwnd: HWND,
    muted: bool,
    overlay: &OverlayConfig,
    duration_ms: u32,
) {
    native_overlay::update(muted, overlay);
    native_overlay::show();
    unsafe {
        let _ = KillTimer(hwnd, ID_OVERLAY_HIDE_TIMER);
        let _ = SetTimer(hwnd, ID_OVERLAY_HIDE_TIMER, duration_ms, None);
    }
}

fn refresh_overlay_displays() {
    let (muted, overlay) = {
        let state = STATE.lock().unwrap();
        (state.muted, state.overlay.clone())
    };
    native_overlay::update(muted, &overlay);
    apply_overlay_visibility();
}
