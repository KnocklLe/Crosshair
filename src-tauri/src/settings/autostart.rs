//! "Launch at Windows startup" through the per-user Run key
//! (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). No administrator
//! rights are required. The NSIS uninstaller removes the same value.

use crate::error::{AppError, AppResult};

pub const RUN_VALUE_NAME: &str = "CrosshairStudio";
/// Argument added to the command line so the app knows it was started by Windows.
pub const AUTOSTART_ARG: &str = "--autostart";

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn command_line() -> AppResult<String> {
        let exe = std::env::current_exe()?;
        Ok(format!("\"{}\" {AUTOSTART_ARG}", exe.display()))
    }

    pub fn set_enabled(enabled: bool) -> AppResult<()> {
        let key = HSTRING::from(RUN_KEY);
        let name = HSTRING::from(RUN_VALUE_NAME);
        unsafe {
            if enabled {
                let value: Vec<u16> = command_line()?.encode_utf16().chain(std::iter::once(0)).collect();
                let status = RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key.as_ptr()),
                    PCWSTR(name.as_ptr()),
                    REG_SZ.0,
                    Some(value.as_ptr() as *const _),
                    (value.len() * 2) as u32,
                );
                if status != ERROR_SUCCESS {
                    return Err(AppError::System(format!("Could not enable launch at startup (error {})", status.0)));
                }
            } else {
                let status = RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), PCWSTR(name.as_ptr()));
                if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
                    return Err(AppError::System(format!("Could not disable launch at startup (error {})", status.0)));
                }
            }
        }
        Ok(())
    }

    /// True when the Run value exists and points to this executable.
    pub fn is_enabled() -> bool {
        let key = HSTRING::from(RUN_KEY);
        let name = HSTRING::from(RUN_VALUE_NAME);
        let mut buf = vec![0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut size),
            )
        };
        if status != ERROR_SUCCESS {
            return false;
        }
        let len = (size as usize / 2).saturating_sub(1).min(buf.len());
        let value = String::from_utf16_lossy(&buf[..len]);
        match std::env::current_exe() {
            Ok(exe) => value.to_lowercase().contains(&exe.display().to_string().to_lowercase()),
            Err(_) => !value.is_empty(),
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn set_enabled(_enabled: bool) -> AppResult<()> {
        Err(AppError::System("Launch at startup is only available on Windows".into()))
    }
    pub fn is_enabled() -> bool {
        false
    }
}

pub use imp::{is_enabled, set_enabled};

/// True when this process was started by the Run key.
pub fn launched_at_startup() -> bool {
    std::env::args().any(|a| a == AUTOSTART_ARG)
}
