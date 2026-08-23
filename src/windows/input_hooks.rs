#![cfg(windows)]

use crate::app::HotkeyAction;
use crate::shortcuts::{binding_virtual_key, ShortcutBinding};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
};

const CTRL_MASK: u8 = 1 << 0;
const SHIFT_MASK: u8 = 1 << 1;
const ALT_MASK: u8 = 1 << 2;
const WIN_MASK: u8 = 1 << 3;
const INPUT_MASK: u32 = 0xff;
const MODIFIER_SHIFT: u32 = 8;
const MODIFIER_MASK: u32 = 0x0f << MODIFIER_SHIFT;
const MOUSE_BINDING: u32 = 1 << 31;

static ACCEPT_BINDING: AtomicU32 = AtomicU32::new(0);
static DECLINE_BINDING: AtomicU32 = AtomicU32::new(0);
static ACTION: AtomicU32 = AtomicU32::new(0);
static TIMING_ENABLED: AtomicBool = AtomicBool::new(false);
static CALLBACK_BUCKETS: [AtomicU64; 6] = [const { AtomicU64::new(0) }; 6];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompiledBindings {
    accept: u32,
    decline: u32,
}

impl CompiledBindings {
    fn new(accept: &ShortcutBinding, decline: &ShortcutBinding) -> Self {
        Self {
            accept: compile_binding(accept),
            decline: compile_binding(decline),
        }
    }

    const fn needs_keyboard(self) -> bool {
        self.accept & MOUSE_BINDING == 0 || self.decline & MOUSE_BINDING == 0
    }

    const fn needs_mouse(self) -> bool {
        self.accept & MOUSE_BINDING != 0 || self.decline & MOUSE_BINDING != 0
    }

    const fn has_keyboard_key(self, virtual_key: u32) -> bool {
        binding_has_keyboard_key(self.accept, virtual_key)
            || binding_has_keyboard_key(self.decline, virtual_key)
    }

    const fn requires_modifier_state(self, virtual_key: u32) -> bool {
        binding_requires_modifier_state(self.accept, virtual_key)
            || binding_requires_modifier_state(self.decline, virtual_key)
    }

    fn action_for_keyboard(self, virtual_key: u32, modifiers: u8) -> Option<HotkeyAction> {
        action_for_match(
            binding_matches_keyboard(self.accept, virtual_key, modifiers),
            binding_matches_keyboard(self.decline, virtual_key, modifiers),
        )
    }

    fn action_for_mouse(self, button: u8) -> Option<HotkeyAction> {
        action_for_match(
            binding_matches_mouse(self.accept, button),
            binding_matches_mouse(self.decline, button),
        )
    }
}

fn action_for_match(accept: bool, decline: bool) -> Option<HotkeyAction> {
    if accept {
        Some(HotkeyAction::Accept)
    } else if decline {
        Some(HotkeyAction::Decline)
    } else {
        None
    }
}

fn compile_binding(binding: &ShortcutBinding) -> u32 {
    if let Some(button) = mouse_button(&binding.input) {
        return MOUSE_BINDING | u32::from(button);
    }
    let virtual_key = binding_virtual_key(&binding.input).unwrap_or_default();
    virtual_key | (u32::from(modifier_mask(&binding.modifiers)) << MODIFIER_SHIFT)
}

fn modifier_mask(modifiers: &[String]) -> u8 {
    modifiers.iter().fold(0, |mask, modifier| {
        mask | match modifier.as_str() {
            "ctrl" => CTRL_MASK,
            "shift" => SHIFT_MASK,
            "alt" => ALT_MASK,
            "win" => WIN_MASK,
            _ => 0,
        }
    })
}

fn mouse_button(input: &str) -> Option<u8> {
    match input {
        "MOUSE1" => Some(1),
        "MOUSE2" => Some(2),
        "MOUSE3" => Some(3),
        "MOUSE4" => Some(4),
        "MOUSE5" => Some(5),
        _ => None,
    }
}

const fn binding_matches_keyboard(binding: u32, virtual_key: u32, modifiers: u8) -> bool {
    if !binding_has_keyboard_key(binding, virtual_key) {
        return false;
    }
    let expected_modifiers = ((binding & MODIFIER_MASK) >> MODIFIER_SHIFT) as u8;
    modifiers & expected_modifiers == expected_modifiers
}

const fn binding_has_keyboard_key(binding: u32, virtual_key: u32) -> bool {
    binding != 0 && binding & MOUSE_BINDING == 0 && binding & INPUT_MASK == virtual_key
}

const fn binding_requires_modifier_state(binding: u32, virtual_key: u32) -> bool {
    binding_has_keyboard_key(binding, virtual_key) && binding & MODIFIER_MASK != 0
}

const fn binding_matches_mouse(binding: u32, button: u8) -> bool {
    binding & MOUSE_BINDING != 0 && binding & INPUT_MASK == button as u32
}

fn active_bindings() -> CompiledBindings {
    CompiledBindings {
        accept: ACCEPT_BINDING.load(Ordering::Acquire),
        decline: DECLINE_BINDING.load(Ordering::Acquire),
    }
}

fn publish_bindings(bindings: CompiledBindings) {
    ACCEPT_BINDING.store(bindings.accept, Ordering::Release);
    DECLINE_BINDING.store(bindings.decline, Ordering::Release);
}

fn clear_bindings() {
    ACCEPT_BINDING.store(0, Ordering::Release);
    DECLINE_BINDING.store(0, Ordering::Release);
}

fn queue_action(action: HotkeyAction) {
    let value = match action {
        HotkeyAction::Accept => 1,
        HotkeyAction::Decline => 2,
    };
    let _ = ACTION.compare_exchange(0, value, Ordering::AcqRel, Ordering::Acquire);
}

pub fn take_action() -> Option<HotkeyAction> {
    match ACTION.swap(0, Ordering::AcqRel) {
        1 => Some(HotkeyAction::Accept),
        2 => Some(HotkeyAction::Decline),
        _ => None,
    }
}

pub fn install() -> bool {
    let (accept, decline) = crate::windows::startup::load_bindings();
    install_bindings(&accept, &decline)
}

pub fn check_registration_lifecycle() -> Result<(bool, bool), &'static str> {
    let (accept, decline) = crate::windows::startup::load_bindings();
    let bindings = CompiledBindings::new(&accept, &decline);
    let expected = (bindings.needs_keyboard(), bindings.needs_mouse());
    if !install_bindings(&accept, &decline) {
        return Err("could not install configured hooks");
    }
    if installed_hook_types() != expected {
        uninstall();
        return Err("installed hook types did not match configured devices");
    }
    if !install_bindings(&accept, &decline) {
        return Err("could not replace configured hooks");
    }
    if installed_hook_types() != expected {
        uninstall();
        return Err("replacement hook types did not match configured devices");
    }
    uninstall();
    if installed_hook_types() != (false, false) {
        return Err("configured hooks remained installed after cleanup");
    }
    Ok(expected)
}

fn install_bindings(accept: &ShortcutBinding, decline: &ShortcutBinding) -> bool {
    uninstall();
    let bindings = CompiledBindings::new(accept, decline);
    publish_bindings(bindings);
    unsafe {
        if bindings.needs_keyboard() {
            KEY_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), HINSTANCE::default(), 0)
                .unwrap_or_default();
        }
        if bindings.needs_mouse() {
            MOUSE_HOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0)
                .unwrap_or_default();
        }
        let keyboard_ready = !bindings.needs_keyboard() || !KEY_HOOK.0.is_null();
        let mouse_ready = !bindings.needs_mouse() || !MOUSE_HOOK.0.is_null();
        if keyboard_ready && mouse_ready {
            return true;
        }
    }
    uninstall();
    false
}

fn installed_hook_types() -> (bool, bool) {
    unsafe { (!KEY_HOOK.0.is_null(), !MOUSE_HOOK.0.is_null()) }
}

pub fn uninstall() {
    ACTION.store(0, Ordering::Release);
    clear_bindings();
    unsafe {
        if !KEY_HOOK.0.is_null() {
            let _ = UnhookWindowsHookEx(KEY_HOOK);
        }
        if !MOUSE_HOOK.0.is_null() {
            let _ = UnhookWindowsHookEx(MOUSE_HOOK);
        }
        KEY_HOOK = HHOOK(std::ptr::null_mut());
        MOUSE_HOOK = HHOOK(std::ptr::null_mut());
    }
}

static mut KEY_HOOK: HHOOK = HHOOK(std::ptr::null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

pub fn run_diagnostic() {
    let (accept, decline) = crate::windows::startup::load_bindings();
    let compiled = CompiledBindings::new(&accept, &decline);
    reset_callback_timing();
    TIMING_ENABLED.store(true, Ordering::Release);
    if !install_bindings(&accept, &decline) {
        TIMING_ENABLED.store(false, Ordering::Release);
        eprintln!("could not install configured input hooks");
        return;
    }
    println!(
        "input hook diagnostic active for 30 seconds: keyboard={} mouse={}",
        compiled.needs_keyboard(),
        compiled.needs_mouse()
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let mut message = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        unsafe {
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                DispatchMessageW(&message);
                if Instant::now() >= deadline {
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    uninstall();
    TIMING_ENABLED.store(false, Ordering::Release);
    let buckets = callback_timing();
    println!(
        "input hook diagnostic complete; callback histogram <1us={} <5us={} <10us={} <50us={} <100us={} >=100us={}",
        buckets[0], buckets[1], buckets[2], buckets[3], buckets[4], buckets[5]
    );
}

fn reset_callback_timing() {
    for bucket in &CALLBACK_BUCKETS {
        bucket.store(0, Ordering::Release);
    }
}

fn callback_timing() -> [u64; 6] {
    std::array::from_fn(|index| CALLBACK_BUCKETS[index].load(Ordering::Acquire))
}

fn record_callback_duration(started: Option<Instant>) {
    let Some(started) = started else {
        return;
    };
    let micros = started.elapsed().as_micros();
    let bucket = match micros {
        0 => 0,
        1..=4 => 1,
        5..=9 => 2,
        10..=49 => 3,
        50..=99 => 4,
        _ => 5,
    };
    CALLBACK_BUCKETS[bucket].fetch_add(1, Ordering::Relaxed);
}

unsafe extern "system" fn key_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let started = TIMING_ENABLED.load(Ordering::Relaxed).then(Instant::now);
    if code >= 0 && wparam.0 as u32 == WM_KEYDOWN {
        let data = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let bindings = active_bindings();
        if bindings.has_keyboard_key(data.vkCode) {
            let mut modifiers = 0;
            if bindings.requires_modifier_state(data.vkCode) {
                if GetAsyncKeyState(0x11) < 0 {
                    modifiers |= CTRL_MASK;
                }
                if GetAsyncKeyState(0x10) < 0 {
                    modifiers |= SHIFT_MASK;
                }
                if GetAsyncKeyState(0x12) < 0 {
                    modifiers |= ALT_MASK;
                }
                if GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 {
                    modifiers |= WIN_MASK;
                }
            }
            if let Some(action) = bindings.action_for_keyboard(data.vkCode, modifiers) {
                queue_action(action);
            }
        }
    }
    record_callback_duration(started);
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let started = TIMING_ENABLED.load(Ordering::Relaxed).then(Instant::now);
    if code >= 0 {
        let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let button = match wparam.0 as u32 {
            WM_LBUTTONDOWN => Some(1),
            WM_RBUTTONDOWN => Some(2),
            WM_MBUTTONDOWN => Some(3),
            WM_XBUTTONDOWN if (data.mouseData >> 16) == 1 => Some(4),
            WM_XBUTTONDOWN => Some(5),
            WM_MOUSEMOVE => None,
            _ => None,
        };
        if let Some(button) = button {
            if let Some(action) = active_bindings().action_for_mouse(button) {
                queue_action(action);
            }
        }
    }
    record_callback_duration(started);
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::ShortcutBindings;

    fn binding(value: &str) -> ShortcutBinding {
        ShortcutBinding::parse(value).unwrap()
    }

    #[test]
    fn compiled_keyboard_matching_preserves_existing_semantics() {
        let legacy = ShortcutBindings::new(binding("Ctrl+Shift+C"), binding("Alt+F5")).unwrap();
        let compiled = CompiledBindings::new(&legacy.accept, &legacy.decline);

        for (virtual_key, legacy_modifiers, compiled_modifiers) in [
            ('C' as u32, vec!["ctrl", "shift"], CTRL_MASK | SHIFT_MASK),
            (
                'C' as u32,
                vec!["ctrl", "shift", "alt"],
                CTRL_MASK | SHIFT_MASK | ALT_MASK,
            ),
            ('C' as u32, vec!["ctrl"], CTRL_MASK),
            (0x74, vec!["alt"], ALT_MASK),
            (0x74, vec![], 0),
        ] {
            assert_eq!(
                compiled.action_for_keyboard(virtual_key, compiled_modifiers),
                legacy.action_for_keyboard(virtual_key, &legacy_modifiers)
            );
        }
    }

    #[test]
    fn compiled_mouse_matching_preserves_existing_semantics() {
        let legacy = ShortcutBindings::new(binding("Mouse4"), binding("Ctrl+Mouse5")).unwrap();
        let compiled = CompiledBindings::new(&legacy.accept, &legacy.decline);

        assert_eq!(
            compiled.action_for_mouse(4),
            legacy.action_for_mouse("mouse4")
        );
        assert_eq!(
            compiled.action_for_mouse(5),
            legacy.action_for_mouse("mouse5")
        );
        assert_eq!(compiled.action_for_mouse(3), None);
    }

    #[test]
    fn required_hook_types_follow_binding_devices() {
        let keyboard = CompiledBindings::new(&binding("F1"), &binding("Ctrl+A"));
        assert!(keyboard.needs_keyboard());
        assert!(!keyboard.needs_mouse());

        let mouse = CompiledBindings::new(&binding("Mouse4"), &binding("Mouse5"));
        assert!(!mouse.needs_keyboard());
        assert!(mouse.needs_mouse());

        let mixed = CompiledBindings::new(&binding("F1"), &binding("Mouse5"));
        assert!(mixed.needs_keyboard());
        assert!(mixed.needs_mouse());
    }

    #[test]
    fn unrelated_keys_skip_modifier_state_queries() {
        let bindings = CompiledBindings::new(&binding("Ctrl+C"), &binding("F2"));
        assert!(!bindings.has_keyboard_key('A' as u32));
        assert!(!bindings.requires_modifier_state('A' as u32));
        assert!(bindings.has_keyboard_key('C' as u32));
        assert!(bindings.requires_modifier_state('C' as u32));
        assert!(bindings.has_keyboard_key(0x71));
        assert!(!bindings.requires_modifier_state(0x71));
    }

    #[test]
    fn callback_timing_buckets_reset_cleanly() {
        CALLBACK_BUCKETS[0].store(3, Ordering::Release);
        CALLBACK_BUCKETS[5].store(2, Ordering::Release);
        reset_callback_timing();
        assert_eq!(callback_timing(), [0; 6]);
    }
}
