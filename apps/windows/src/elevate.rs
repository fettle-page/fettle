//! The Deep check: Fettle restarts itself as administrator with `--deep`, and the new window checks
//! straight away. The scan reads more when it runs elevated (disk health, the TPM chip).

use crate::state::DeepError;

/// The argument the elevated copy of the app starts with.
pub const DEEP_ARG: &str = "--deep";

/// Whether this copy of the app was started for a Deep check.
pub fn wants_deep(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().skip(1).any(|a| a == DEEP_ARG)
}

/// The arguments the elevated copy starts with: a Deep check, and which process to wait for.
pub fn elevated_params(parent: u32) -> String {
    format!("{DEEP_ARG} --parent {parent}")
}

/// The process the elevated copy waits for (`--parent <pid>`), if any.
pub fn parent_pid(args: impl IntoIterator<Item = String>) -> Option<u32> {
    let mut args = args.into_iter().skip(1);
    while let Some(a) = args.next() {
        if a == "--parent" {
            return args.next()?.parse().ok();
        }
    }
    None
}

/// The elevated copy waits (up to 5 s) for the window that started it to close, so the two never
/// write Fettle's settings and last check at the same time.
#[cfg(windows)]
pub fn wait_for_parent(pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    // SAFETY: the handle is checked, waited on and closed here; a gone process just fails to open.
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            WaitForSingleObject(handle, 5_000);
            let _ = CloseHandle(handle);
        }
    }
}

/// ERROR_CANCELLED (1223) as an HRESULT: the user declined the prompt.
const CANCELLED_HRESULT: i32 = 0x8007_04C7_u32 as i32;

pub fn error_from(hresult: i32) -> DeepError {
    if hresult == CANCELLED_HRESULT { DeepError::Cancelled } else { DeepError::Failed }
}

/// Starts this exe again as administrator with `--deep`, and returns once Windows has started it
/// (or the user declined). Windows' prompt belongs to `owner` (the app's window, 0 for none). The
/// caller closes this window afterwards.
#[cfg(windows)]
pub fn start_elevated_copy(owner: isize) -> Result<(), DeepError> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::{HSTRING, PCWSTR, w};

    let exe = std::env::current_exe().map_err(|_| DeepError::Failed)?;
    let file = HSTRING::from(exe.as_os_str());
    let params = HSTRING::from(elevated_params(std::process::id()));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC,
        hwnd: HWND(owner as _),
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` is fully initialised and the strings it points to outlive the call.
    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| error_from(e.code().0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn only_the_deep_argument_asks_for_a_deep_check() {
        assert!(wants_deep(args(&["fettle.exe", "--deep"])));
        assert!(!wants_deep(args(&["fettle.exe"])));
        assert!(!wants_deep(args(&["fettle.exe", "--deeper", "deep"])));
        // The program's own path never counts, even if a folder is called "--deep".
        assert!(!wants_deep(args(&["--deep"])));
    }

    #[test]
    fn the_elevated_copy_is_told_which_window_to_wait_for() {
        assert_eq!(elevated_params(4242), "--deep --parent 4242");
        let params = elevated_params(4242);
        let started_with = std::iter::once("fettle.exe".to_string()).chain(params.split(' ').map(String::from));
        assert!(wants_deep(started_with));
        assert_eq!(parent_pid(args(&["fettle.exe", "--deep", "--parent", "4242"])), Some(4242));
        assert_eq!(parent_pid(args(&["fettle.exe", "--deep"])), None);
        assert_eq!(parent_pid(args(&["fettle.exe", "--parent", "nope"])), None);
        assert_eq!(parent_pid(args(&["fettle.exe", "--parent"])), None);
    }

    #[test]
    fn a_declined_prompt_is_cancelled_and_anything_else_failed() {
        assert_eq!(error_from(0x8007_04C7_u32 as i32), DeepError::Cancelled);
        assert_eq!(error_from(0x8007_0005_u32 as i32), DeepError::Failed); // access denied
    }

}
