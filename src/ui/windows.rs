use anyhow::Result;

use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn set_visible(pid: u32, visible: bool) -> Result<()> {
    unsafe {
        if !windows_set_visible(pid, visible) {
            return Err(anyhow::anyhow!("Failed to set visible"));
        }
    }

    Ok(())
}

struct EnumWindowsParameter {
    pid: u32,
    visible: bool,
    result: bool,
}

unsafe fn windows_set_visible(pid: u32, visible: bool) -> bool {
    let parameter = EnumWindowsParameter {
        pid,
        visible,
        result: false,
    };
    let _ = EnumWindows(
        Some(check_and_set_visible),
        LPARAM(&parameter as *const _ as isize),
    );

    parameter.result
}

const TITLE: &str = "ShellProtectorOSC";

unsafe extern "system" fn check_and_set_visible(hwnd: HWND, param: LPARAM) -> BOOL {
    let parameter = &mut *(param.0 as *mut EnumWindowsParameter);
    let pid = parameter.pid;
    let visible = parameter.visible;

    let mut window_pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
    if window_pid == pid {
        let title_length = GetWindowTextLengthW(hwnd);
        let mut title = vec![0; title_length as usize + 1];
        GetWindowTextW(hwnd, &mut title);
        let title = String::from_utf16_lossy(&title[..title_length as usize])
            .trim()
            .to_string();
        if title == TITLE {
            parameter.result =
                ShowWindow(hwnd, if visible { SW_NORMAL } else { SW_HIDE }).as_bool();
            return BOOL(0);
        }
    }

    BOOL(1)
}
