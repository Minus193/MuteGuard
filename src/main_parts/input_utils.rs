fn cleanup() {
    shutdown_audio_notification_registration();
    native_overlay::destroy();
    remove_tray_icon();
    let (hook, mouse_hook) = {
        let state = STATE.lock().unwrap();
        (state.hook, state.mouse_hook)
    };
    if !hook.0.is_null() {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    }
    if !mouse_hook.0.is_null() {
        unsafe {
            let _ = UnhookWindowsHookEx(mouse_hook);
        }
    }
}

fn key_down(vk: u32) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

fn canonical_keyboard_vk(vk: u32) -> u32 {
    match modifier_kind(vk) {
        Some(ModifierKind::Ctrl) => VK_CONTROL,
        Some(ModifierKind::Alt) => VK_MENU,
        Some(ModifierKind::Shift) => VK_SHIFT,
        Some(ModifierKind::Win) => VK_LWIN,
        None => vk,
    }
}

fn keyboard_key_down(vk: u32) -> bool {
    match canonical_keyboard_vk(vk) {
        VK_CONTROL => key_down(VK_CONTROL) || key_down(0xA2) || key_down(0xA3),
        VK_MENU => key_down(VK_MENU) || key_down(0xA4) || key_down(0xA5),
        VK_SHIFT => key_down(VK_SHIFT) || key_down(0xA0) || key_down(0xA1),
        VK_LWIN => key_down(VK_LWIN) || key_down(VK_RWIN),
        key => key_down(key),
    }
}

fn keyboard_key_sort_key(vk: u32) -> (u8, u32) {
    match canonical_keyboard_vk(vk) {
        VK_CONTROL => (0, 0),
        VK_MENU => (1, 0),
        VK_SHIFT => (2, 0),
        VK_LWIN => (3, 0),
        key => (4, key),
    }
}

fn sync_modifier_keys_down(keys_down: &mut HashSet<u32>) {
    for modifier in [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN] {
        if keyboard_key_down(modifier) {
            keys_down.insert(modifier);
        } else {
            keys_down.remove(&modifier);
        }
    }
}

#[derive(Clone, Copy)]
enum ModifierKind {
    Ctrl,
    Alt,
    Shift,
    Win,
}

fn modifier_kind(vk: u32) -> Option<ModifierKind> {
    match vk {
        VK_SHIFT | 0xA0 | 0xA1 => Some(ModifierKind::Shift),
        VK_CONTROL | 0xA2 | 0xA3 => Some(ModifierKind::Ctrl),
        VK_MENU | 0xA4 | 0xA5 => Some(ModifierKind::Alt),
        VK_LWIN | VK_RWIN => Some(ModifierKind::Win),
        _ => None,
    }
}

fn mouse_button_from_event(event: u32, mouse_data: u32) -> Option<u32> {
    match event {
        WM_LBUTTONDOWN | WM_LBUTTONUP => Some(VK_LBUTTON),
        WM_RBUTTONDOWN | WM_RBUTTONUP => Some(VK_RBUTTON),
        WM_MBUTTONDOWN | WM_MBUTTONUP => Some(VK_MBUTTON),
        WM_XBUTTONDOWN | WM_XBUTTONUP => match (mouse_data >> 16) & 0xffff {
            XBUTTON1 => Some(VK_XBUTTON1),
            XBUTTON2 => Some(VK_XBUTTON2),
            _ => None,
        },
        _ => None,
    }
}

fn mouse_button_event_is_down(event: u32) -> bool {
    matches!(
        event,
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
    )
}

fn mouse_button_sort_key(button: u32) -> u32 {
    match button {
        VK_LBUTTON => 0,
        VK_RBUTTON => 1,
        VK_MBUTTON => 2,
        VK_XBUTTON1 => 3,
        VK_XBUTTON2 => 4,
        _ => button + 100,
    }
}

pub(crate) fn mouse_button_name(button: u32) -> &'static str {
    match button {
        VK_LBUTTON => "Left Click",
        VK_RBUTTON => "Right Click",
        VK_MBUTTON => "Middle Click",
        VK_XBUTTON1 => "Mouse 4",
        VK_XBUTTON2 => "Mouse 5",
        _ => "Mouse",
    }
}

fn is_supported_mouse_button(button: u32) -> bool {
    matches!(
        button,
        VK_LBUTTON | VK_RBUTTON | VK_MBUTTON | VK_XBUTTON1 | VK_XBUTTON2
    )
}

fn vk_name(vk: u32) -> String {
    match canonical_keyboard_vk(vk) {
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0D => "Enter".to_string(),
        VK_SHIFT => "Shift".to_string(),
        VK_CONTROL => "Ctrl".to_string(),
        VK_MENU => "Alt".to_string(),
        0x13 => "Pause".to_string(),
        0x14 => "Caps Lock".to_string(),
        0x1B => "Esc".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "Page Up".to_string(),
        0x22 => "Page Down".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2C => "Print Screen".to_string(),
        0x2D => "Insert".to_string(),
        0x2E => "Delete".to_string(),
        VK_LWIN => "Win".to_string(),
        0x5D => "Menu".to_string(),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk).unwrap().to_string(),
        VK_NUMPAD0..=0x69 => format!("Numpad {}", vk - VK_NUMPAD0),
        0x6A => "Numpad *".to_string(),
        0x6B => "Numpad +".to_string(),
        0x6D => "Numpad -".to_string(),
        0x6E => "Numpad .".to_string(),
        0x6F => "Numpad /".to_string(),
        VK_F1..=0x87 => format!("F{}", vk - VK_F1 + 1),
        0x90 => "Num Lock".to_string(),
        0x91 => "Scroll Lock".to_string(),
        0xA6 => "Browser Back".to_string(),
        0xA7 => "Browser Forward".to_string(),
        0xA8 => "Browser Refresh".to_string(),
        0xA9 => "Browser Stop".to_string(),
        0xAA => "Browser Search".to_string(),
        0xAB => "Browser Favorites".to_string(),
        0xAC => "Browser Home".to_string(),
        0xAD => "Volume Mute".to_string(),
        0xAE => "Volume Down".to_string(),
        0xAF => "Volume Up".to_string(),
        0xB0 => "Next Track".to_string(),
        0xB1 => "Previous Track".to_string(),
        0xB2 => "Media Stop".to_string(),
        0xB3 => "Play/Pause".to_string(),
        0xB4 => "Mail".to_string(),
        0xB5 => "Media Player".to_string(),
        0xB6 => "App 1".to_string(),
        0xB7 => "App 2".to_string(),
        0xBA => ";".to_string(),
        0xBB => "=".to_string(),
        0xBC => ",".to_string(),
        0xBD => "-".to_string(),
        0xBE => ".".to_string(),
        0xBF => "/".to_string(),
        0xC0 => "`".to_string(),
        0xDB => "[".to_string(),
        0xDC => "\\".to_string(),
        0xDD => "]".to_string(),
        0xDE => "'".to_string(),
        0xE2 => "Intl".to_string(),
        _ => format!("VK {vk}"),
    }
}

fn write_packed_wide_buf<const N: usize>(buf: *mut [u16; N], text: &str) {
    let wide = wide(text);
    let len = (wide.len() - 1).min(N - 1);
    let ptr = buf.cast::<u16>();

    unsafe {
        for (index, value) in wide.iter().take(len).copied().enumerate() {
            std::ptr::write_unaligned(ptr.add(index), value);
        }
        std::ptr::write_unaligned(ptr.add(len), 0);
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
