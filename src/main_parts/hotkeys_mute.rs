fn exit_all_processes() {
    close_settings_window();
    let hwnd = STATE.lock().unwrap().hwnd;
    if hwnd.0.is_null() {
        return;
    }
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let event = wparam.0 as u32;
    let keyboard = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let vk = normalized_keyboard_vk(keyboard.vkCode, keyboard.scanCode, keyboard.flags.0);
    let is_down = event == WM_KEYDOWN || event == 0x0104;
    let is_up = event == WM_KEYUP || event == 0x0105;

    if is_down {
        {
            let mut state = STATE.lock().unwrap();
            state.keyboard_keys_down.insert(vk);
            sync_modifier_keys_down(&mut state.keyboard_keys_down);
        }

        if SETTINGS_HOTKEY_RECORDING.load(Ordering::Relaxed) {
            record_settings_keyboard_event(vk, true);
            return LRESULT(1);
        }

        let (targets, suppress_key) = {
            let mut state = STATE.lock().unwrap();
            let matching = state
                .hotkeys
                .iter()
                .filter(|hotkey| {
                    hotkey.shortcut.is_pressed(
                        vk,
                        hotkey.ignore_modifiers,
                        &state.keyboard_keys_down,
                        &state.mouse_buttons_down,
                    )
                })
                .cloned()
                .collect::<Vec<_>>();
            let exact_shortcuts = matching
                .iter()
                .filter(|hotkey| !hotkey.ignore_modifiers)
                .map(|hotkey| hotkey.shortcut.clone())
                .collect::<Vec<_>>();
            let mut targets = Vec::new();
            let mut suppress_key = false;

            for hotkey in matching {
                if hotkey.ignore_modifiers
                    && exact_shortcuts
                        .iter()
                        .any(|shortcut| shortcut.same_inputs(&hotkey.shortcut))
                {
                    continue;
                }
                suppress_key |= shortcut_is_alt_space(&hotkey.shortcut);
                if state.hotkeys_down.insert(hotkey.id.clone()) {
                    targets.push(hotkey.target.clone());
                }
            }
            (targets, suppress_key)
        };

        queue_mute_targets(targets);
        if suppress_key {
            return LRESULT(1);
        }
    }

    if is_up {
        let mut state = STATE.lock().unwrap();
        state.keyboard_keys_down.remove(&vk);
        sync_modifier_keys_down(&mut state.keyboard_keys_down);
        if SETTINGS_HOTKEY_RECORDING.load(Ordering::Relaxed) {
            drop(state);
            record_settings_keyboard_event(vk, false);
            return LRESULT(1);
        }
        let released = state
            .hotkeys
            .iter()
            .filter(|hotkey| {
                state.hotkeys_down.contains(&hotkey.id)
                    && hotkey.shortcut.contains_keyboard_key(vk)
                    && !hotkey.shortcut.is_held(
                        hotkey.ignore_modifiers,
                        &state.keyboard_keys_down,
                        &state.mouse_buttons_down,
                    )
            })
            .map(|hotkey| hotkey.id.clone())
            .collect::<Vec<_>>();
        for id in released {
            state.hotkeys_down.remove(&id);
        }
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn record_settings_keyboard_event(vk: u32, down: bool) {
    if SETTINGS_PRESSED_SHORTCUT.lock().unwrap().is_some() {
        return;
    }
    SETTINGS_CAPTURE_LAST_EVENT
        .lock()
        .unwrap()
        .replace(Instant::now());
    let vk = canonical_keyboard_vk(vk);
    if down {
        let mut held = SETTINGS_KEYBOARD_HELD.lock().unwrap();
        if !held.contains(&vk) {
            held.push(vk);
        }
        drop(held);
        let mut chord = SETTINGS_KEYBOARD_CHORD.lock().unwrap();
        if !chord.contains(&vk) {
            chord.push(vk);
        }
    } else {
        SETTINGS_KEYBOARD_HELD
            .lock()
            .unwrap()
            .retain(|held| *held != vk);
        finish_settings_shortcut();
    }
}

fn normalized_keyboard_vk(vk: u32, scan_code: u32, flags: u32) -> u32 {
    let vk = canonical_keyboard_vk(vk);
    if flags & LLKHF_EXTENDED != 0 {
        return vk;
    }

    match scan_code {
        0x52 => VK_NUMPAD0,
        0x4F => VK_NUMPAD0 + 1,
        0x50 => VK_NUMPAD0 + 2,
        0x51 => VK_NUMPAD0 + 3,
        0x4B => VK_NUMPAD0 + 4,
        0x4C => VK_NUMPAD0 + 5,
        0x4D => VK_NUMPAD0 + 6,
        0x47 => VK_NUMPAD0 + 7,
        0x48 => VK_NUMPAD0 + 8,
        0x49 => VK_NUMPAD0 + 9,
        0x53 => 0x6E,
        _ => vk,
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let event = wparam.0 as u32;
    let mouse = unsafe { *(lparam.0 as *const MSLLHOOKSTRUCT) };
    let Some(button) = mouse_button_from_event(event, mouse.mouseData) else {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    };
    let down = mouse_button_event_is_down(event);

    let recording_button = SETTINGS_MOUSE_HELD.lock().unwrap().contains(&button);
    if SETTINGS_HOTKEY_RECORDING.load(Ordering::Relaxed)
        && (!point_inside_settings_window(mouse.pt) || recording_button)
    {
        record_settings_mouse_event(button, down);
    }

    if !MOUSE_HOTKEYS_ENABLED.load(Ordering::Relaxed) {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let targets = {
        let mut state = STATE.lock().unwrap();
        sync_modifier_keys_down(&mut state.keyboard_keys_down);
        if down {
            state.mouse_buttons_down.insert(button);
            mouse_press_targets(&mut state, button)
        } else {
            state.mouse_buttons_down.remove(&button);
            release_mouse_hotkeys(&mut state, button);
            Vec::new()
        }
    };

    queue_mute_targets(targets);

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn record_settings_mouse_event(button: u32, down: bool) {
    if SETTINGS_PRESSED_SHORTCUT.lock().unwrap().is_some() {
        return;
    }
    SETTINGS_CAPTURE_LAST_EVENT
        .lock()
        .unwrap()
        .replace(Instant::now());
    if down {
        let mut held = SETTINGS_MOUSE_HELD.lock().unwrap();
        if !held.contains(&button) {
            held.push(button);
        }
        drop(held);
        let mut chord = SETTINGS_MOUSE_CHORD.lock().unwrap();
        if !chord.contains(&button) {
            chord.push(button);
        }
    } else {
        SETTINGS_MOUSE_HELD
            .lock()
            .unwrap()
            .retain(|held| *held != button);
        finish_settings_shortcut();
    }
}

fn finalize_settings_shortcut_if_released() {
    if !SETTINGS_KEYBOARD_HELD.lock().unwrap().is_empty()
        || !SETTINGS_MOUSE_HELD.lock().unwrap().is_empty()
    {
        return;
    }

    finish_settings_shortcut();
}

fn poll_settings_shortcut_capture() {
    if !SETTINGS_HOTKEY_RECORDING.load(Ordering::Relaxed)
        || SETTINGS_PRESSED_SHORTCUT.lock().unwrap().is_some()
    {
        return;
    }

    let mut keyboard_down = Vec::new();
    for vk in 0x03..=0xFE {
        if is_supported_mouse_button(vk) || !key_down(vk) {
            continue;
        }
        let vk = canonical_keyboard_vk(vk);
        if !keyboard_down.contains(&vk) {
            keyboard_down.push(vk);
        }
    }

    let previous_keyboard = SETTINGS_KEYBOARD_HELD.lock().unwrap().clone();
    let previous_mouse = SETTINGS_MOUSE_HELD.lock().unwrap().clone();
    let mut cursor = POINT::default();
    let _ = unsafe { GetCursorPos(&mut cursor) };
    let cursor_inside_settings = point_inside_settings_window(cursor);
    let mouse_down = [
        VK_LBUTTON,
        VK_RBUTTON,
        VK_MBUTTON,
        VK_XBUTTON1,
        VK_XBUTTON2,
    ]
    .into_iter()
    .filter(|button| {
        key_down(*button)
            && (!cursor_inside_settings || previous_mouse.contains(button))
    })
    .collect::<Vec<_>>();

    {
        let mut chord = SETTINGS_KEYBOARD_CHORD.lock().unwrap();
        for vk in &keyboard_down {
            if !chord.contains(vk) {
                chord.push(*vk);
            }
        }
    }
    {
        let mut chord = SETTINGS_MOUSE_CHORD.lock().unwrap();
        for button in &mouse_down {
            if !chord.contains(button) {
                chord.push(*button);
            }
        }
    }

    SETTINGS_KEYBOARD_HELD
        .lock()
        .unwrap()
        .clone_from(&keyboard_down);
    SETTINGS_MOUSE_HELD
        .lock()
        .unwrap()
        .clone_from(&mouse_down);

    if settings_capture_has_release(
        &previous_keyboard,
        &previous_mouse,
        &keyboard_down,
        &mouse_down,
    ) {
        finish_settings_shortcut();
    }
}

fn settings_capture_has_release(
    previous_keyboard: &[u32],
    previous_mouse: &[u32],
    keyboard_down: &[u32],
    mouse_down: &[u32],
) -> bool {
    previous_keyboard
        .iter()
        .any(|vk| !keyboard_down.contains(vk))
        || previous_mouse
            .iter()
            .any(|button| !mouse_down.contains(button))
}

fn finish_settings_shortcut() {
    let keyboard_keys = std::mem::take(&mut *SETTINGS_KEYBOARD_CHORD.lock().unwrap());
    let mut mouse_buttons = std::mem::take(&mut *SETTINGS_MOUSE_CHORD.lock().unwrap());
    mouse_buttons.truncate(2);
    if keyboard_keys.is_empty() && mouse_buttons.is_empty() {
        return;
    }
    SETTINGS_PRESSED_SHORTCUT
        .lock()
        .unwrap()
        .replace(Shortcut::from_inputs(keyboard_keys, mouse_buttons));
    SETTINGS_CAPTURE_LAST_EVENT.lock().unwrap().take();
}

fn reconcile_settings_capture() {
    let capture_is_settled = SETTINGS_CAPTURE_LAST_EVENT
        .lock()
        .unwrap()
        .is_some_and(|last_event| last_event.elapsed() >= Duration::from_millis(250));
    if !capture_is_settled {
        return;
    }
    SETTINGS_KEYBOARD_HELD
        .lock()
        .unwrap()
        .retain(|key| keyboard_key_down(*key));
    SETTINGS_MOUSE_HELD
        .lock()
        .unwrap()
        .retain(|button| key_down(*button));
    finalize_settings_shortcut_if_released();
}

fn point_inside_settings_window(point: POINT) -> bool {
    let title = wide(SETTINGS_WINDOW_TITLE);
    let Ok(hwnd) = (unsafe { FindWindowW(PCWSTR(null()), PCWSTR(title.as_ptr())) }) else {
        return false;
    };
    if hwnd.0.is_null() {
        return false;
    }

    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok()
        && point.x >= rect.left
        && point.x < rect.right
        && point.y >= rect.top
        && point.y < rect.bottom
}

fn mouse_press_targets(state: &mut AppState, button: u32) -> Vec<Option<String>> {
    let mut matches = state
        .hotkeys
        .iter()
        .filter(|hotkey| {
            hotkey.shortcut.is_pressed(
                button,
                hotkey.ignore_modifiers,
                &state.keyboard_keys_down,
                &state.mouse_buttons_down,
            )
        })
        .cloned()
        .collect::<Vec<_>>();

    let has_combo_match = matches
        .iter()
        .any(|hotkey| hotkey.shortcut.mouse_buttons.len() > 1);
    matches.retain(|hotkey| !has_combo_match || hotkey.shortcut.mouse_buttons.len() > 1);

    matches
        .into_iter()
        .filter_map(|hotkey| {
            state
                .hotkeys_down
                .insert(hotkey.id)
                .then_some(hotkey.target)
        })
        .collect()
}

fn release_mouse_hotkeys(state: &mut AppState, button: u32) {
    let released = state
        .hotkeys
        .iter()
        .filter(|hotkey| {
            state.hotkeys_down.contains(&hotkey.id)
                && hotkey.shortcut.mouse_buttons.contains(&button)
                && !hotkey.shortcut.is_held(
                    hotkey.ignore_modifiers,
                    &state.keyboard_keys_down,
                    &state.mouse_buttons_down,
                )
        })
        .map(|hotkey| hotkey.id.clone())
        .collect::<Vec<_>>();
    for id in released {
        state.hotkeys_down.remove(&id);
    }
}

fn toggle_mute() {
    toggle_mute_target(None);
}

fn queue_mute_targets(targets: Vec<Option<String>>) {
    let targets = unique_mute_targets(targets);
    if targets.is_empty() {
        return;
    }

    let hwnd = {
        let mut state = STATE.lock().unwrap();
        for target in targets {
            state.pending_mute_targets.push_back(target);
        }
        state.hwnd
    };
    if !hwnd.0.is_null() {
        unsafe {
            let _ = SetTimer(
                hwnd,
                ID_HOTKEY_RECONCILE_TIMER,
                HOTKEY_RECONCILE_MS,
                None,
            );
            let _ = PostMessageW(hwnd, WM_PROCESS_HOTKEY_ACTIONS, WPARAM(0), LPARAM(0));
        }
    }
}

fn reconcile_hotkeys_down() {
    let (hwnd, has_active_hotkeys) = {
        let mut state = STATE.lock().unwrap();
        state
            .keyboard_keys_down
            .retain(|key| keyboard_key_down(*key));
        sync_modifier_keys_down(&mut state.keyboard_keys_down);
        let physically_down = state
            .hotkeys
            .iter()
            .filter(|hotkey| shortcut_primary_input_is_down(&hotkey.shortcut))
            .map(|hotkey| hotkey.id.clone())
            .collect::<HashSet<_>>();
        state.hotkeys_down.retain(|id| physically_down.contains(id));
        (state.hwnd, !state.hotkeys_down.is_empty())
    };

    if !has_active_hotkeys {
        unsafe {
            let _ = KillTimer(hwnd, ID_HOTKEY_RECONCILE_TIMER);
        }
    }
}

fn shortcut_primary_input_is_down(shortcut: &Shortcut) -> bool {
    let keyboard_keys = shortcut.keyboard_keys();
    (!keyboard_keys.is_empty() || !shortcut.mouse_buttons.is_empty())
        && keyboard_keys.iter().all(|key| keyboard_key_down(*key))
        && shortcut
            .mouse_buttons
            .iter()
            .all(|button| key_down(*button))
}

fn unique_mute_targets(targets: Vec<Option<String>>) -> Vec<Option<String>> {
    if targets
        .iter()
        .any(|target| target.as_deref() == Some(HOTKEY_TARGET_ALL_MICROPHONES))
    {
        return vec![Some(HOTKEY_TARGET_ALL_MICROPHONES.to_string())];
    }

    let mut unique = Vec::new();
    for target in targets {
        if !unique.contains(&target) {
            unique.push(target);
        }
    }
    unique
}

#[cfg(test)]
mod hotkey_action_tests {
    use super::*;

    #[test]
    fn all_microphones_supersedes_default_and_duplicate_targets() {
        let all = Some(HOTKEY_TARGET_ALL_MICROPHONES.to_string());
        assert_eq!(
            unique_mute_targets(vec![None, None, all.clone(), all.clone()]),
            vec![all]
        );
    }

    #[test]
    fn duplicate_default_targets_are_processed_once() {
        assert_eq!(unique_mute_targets(vec![None, None, None]), vec![None]);
    }

    #[test]
    fn settings_capture_accepts_arbitrary_keyboard_chords_without_ctrl() {
        let shortcut = Shortcut::from_inputs(vec![VK_LWIN, VK_SHIFT, b'A' as u32], Vec::new());

        assert_eq!(
            shortcut.keyboard_keys(),
            vec![VK_SHIFT, VK_LWIN, b'A' as u32]
        );
        assert!(!shortcut.keyboard_keys().contains(&VK_CONTROL));
        assert_eq!(shortcut.display(), "Shift + Win + A");
    }

    #[test]
    fn settings_capture_accepts_non_modifier_and_modifier_only_chords() {
        assert_eq!(
            Shortcut::from_inputs(vec![b'A' as u32, b'B' as u32], Vec::new()).display(),
            "A + B"
        );
        assert_eq!(
            Shortcut::from_inputs(vec![VK_LWIN], Vec::new()).display(),
            "Win"
        );
        assert_eq!(
            Shortcut::from_inputs(vec![0x1B], Vec::new()).display(),
            "Esc"
        );
    }

    #[test]
    fn settings_capture_finishes_when_any_chord_member_is_released() {
        assert!(settings_capture_has_release(
            &[VK_SHIFT, VK_LWIN, b'A' as u32],
            &[],
            &[VK_SHIFT, VK_LWIN],
            &[],
        ));
        assert!(!settings_capture_has_release(
            &[VK_SHIFT, VK_LWIN, b'A' as u32],
            &[],
            &[VK_SHIFT, VK_LWIN, b'A' as u32],
            &[],
        ));
    }
}

fn process_queued_mute_targets() {
    let targets = {
        let mut state = STATE.lock().unwrap();
        state.pending_mute_targets.drain(..).collect::<Vec<_>>()
    };
    for target in targets {
        toggle_mute_target(target.as_deref());
    }
}

fn toggle_mute_target(device_id: Option<&str>) {
    match set_mute_to_inverse(device_id) {
        Ok(()) => refresh_mute_state(),
        Err(error) => {
            refresh_mute_state();
            report_runtime_error(
                "MuteGuard could not change the microphone state",
                format!("{error:#}"),
            );
        }
    }
}

fn apply_startup_auto_mute() {
    let (pending, hwnd) = {
        let state = STATE.lock().unwrap();
        (state.startup_mute_pending, state.hwnd)
    };
    if !pending {
        unsafe {
            let _ = KillTimer(hwnd, ID_STARTUP_MUTE_RETRY_TIMER);
        }
        return;
    }
    match set_mute(None, true) {
        Ok(()) => {
            let hwnd = {
                let mut state = STATE.lock().unwrap();
                state.startup_mute_pending = false;
                state.hwnd
            };
            unsafe {
                let _ = KillTimer(hwnd, ID_STARTUP_MUTE_RETRY_TIMER);
            }
            refresh_mute_state();
        }
        Err(error) => {
            unsafe {
                let _ = SetTimer(
                    hwnd,
                    ID_STARTUP_MUTE_RETRY_TIMER,
                    STARTUP_MUTE_RETRY_MS,
                    None,
                );
            }
            report_audio_error("Startup mute is waiting for a microphone", &error);
        }
    }
}

fn set_global_mute_state(muted: bool, trigger_overlay: bool) {
    let (changed, availability_changed, visibility, duration_secs, sound_feedback) = {
        let mut state = STATE.lock().unwrap();
        let changed = state.muted != muted;
        let availability_changed = !state.audio_available;
        state.muted = muted;
        state.audio_available = true;
        (
            changed,
            availability_changed,
            state.overlay.visibility.clone(),
            state.overlay.duration_secs,
            state.sound_feedback.clone(),
        )
    };

    if !changed && !availability_changed {
        return;
    }
    refresh_tray_icon();
    if changed {
        play_sound_feedback(muted, &sound_feedback);
    }
    if changed && trigger_overlay && visibility == "AfterToggle" {
        show_overlay_temporarily((duration_secs.clamp(0.5, 10.0) * 1_000.0) as u32);
    } else {
        apply_overlay_visibility();
    }
}
