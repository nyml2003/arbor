use std::mem::size_of;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_ESCAPE, VK_MENU, VK_RETURN, VK_SHIFT,
    VK_SPACE,
};

use crate::app::{InputCommand, KeyCode, Modifier};

use super::error::{PlatformError, PlatformResult};

pub fn send(command: &InputCommand) -> PlatformResult<()> {
    match command {
        InputCommand::CloseApp => Ok(()),
        InputCommand::Text(value) => send_unicode(*value),
        InputCommand::KeyTap(key) => send_key_tap(*key),
        InputCommand::ModifiedKeyTap { modifiers, key } => {
            let mut inputs = Vec::new();
            for modifier in modifiers {
                inputs.push(key_input(modifier_vk(*modifier), KEYBD_EVENT_FLAGS(0)));
            }
            inputs.push(key_input(key_vk(*key), KEYBD_EVENT_FLAGS(0)));
            inputs.push(key_input(key_vk(*key), KEYEVENTF_KEYUP));
            for modifier in modifiers.iter().rev() {
                inputs.push(key_input(modifier_vk(*modifier), KEYEVENTF_KEYUP));
            }
            send_inputs(&inputs)
        }
    }
}

fn send_unicode(value: char) -> PlatformResult<()> {
    let unit = value as u16;
    let inputs = [
        unicode_input(unit, KEYBD_EVENT_FLAGS(0)),
        unicode_input(unit, KEYEVENTF_KEYUP),
    ];
    send_inputs(&inputs)
}

fn send_key_tap(key: KeyCode) -> PlatformResult<()> {
    let vk = key_vk(key);
    let inputs = [
        key_input(vk, KEYBD_EVENT_FLAGS(0)),
        key_input(vk, KEYEVENTF_KEYUP),
    ];
    send_inputs(&inputs)
}

fn send_inputs(inputs: &[INPUT]) -> PlatformResult<()> {
    // SAFETY: INPUT slice points to initialized INPUT values and cbSize is exactly INPUT size.
    // SendInput copies the events during the call; the slice lives for the full call duration.
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(PlatformError::PartialInput {
            sent,
            expected: inputs.len(),
        })
    }
}

fn key_input(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn unicode_input(unit: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: KEYEVENTF_UNICODE | flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn modifier_vk(modifier: Modifier) -> VIRTUAL_KEY {
    match modifier {
        Modifier::Shift => VK_SHIFT,
        Modifier::Control => VK_CONTROL,
        Modifier::Alt => VK_MENU,
    }
}

fn key_vk(key: KeyCode) -> VIRTUAL_KEY {
    match key {
        KeyCode::Backspace => VK_BACK,
        KeyCode::Enter => VK_RETURN,
        KeyCode::Escape => VK_ESCAPE,
        KeyCode::Space => VK_SPACE,
        KeyCode::Character(value) => VIRTUAL_KEY(value.to_ascii_uppercase() as u16),
    }
}
