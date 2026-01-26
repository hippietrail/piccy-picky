use libc::{ioctl, isatty, STDOUT_FILENO, TIOCGWINSZ};
use std::path::{Path, PathBuf};

#[repr(C)]
struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

pub fn get_terminal_size() -> (u16, u16) {
    unsafe {
        if isatty(STDOUT_FILENO) == 0 {
            return (80, 24); // Fallback
        }

        let mut ws: WinSize = std::mem::zeroed();
        let ret = ioctl(STDOUT_FILENO, TIOCGWINSZ as u64, &mut ws as *mut WinSize);

        if ret == -1 {
            (80, 24) // Fallback
        } else {
            (ws.ws_col, ws.ws_row)
        }
    }
}

/// Get pixel dimensions of terminal. Some terminals report this via TIOCGWINSZ.
pub fn get_terminal_pixel_size() -> (u32, u32) {
    unsafe {
        let mut ws: WinSize = std::mem::zeroed();
        let ret = ioctl(STDOUT_FILENO, TIOCGWINSZ as u64, &mut ws as *mut WinSize);

        if ret == -1 || ws.ws_xpixel == 0 || ws.ws_ypixel == 0 {
            // Fallback: assume standard macOS Terminal font metrics
            // ~8px width x 16px height per character
            let (cols, rows) = get_terminal_size();
            return ((cols as u32) * 8, (rows as u32) * 16);
        }
        (ws.ws_xpixel as u32, ws.ws_ypixel as u32)
    }
}

/// Abbreviate path to fit terminal width, showing relative path
pub fn abbreviate_path(path: &Path, base_path: &str, max_width: usize) -> String {
    // Try to use relative path
    let rel_path = path
        .strip_prefix(base_path)
        .unwrap_or(path)
        .to_string_lossy();

    let path_str = rel_path.to_string();
    
    // If it fits, return as-is
    if path_str.len() <= max_width {
        return path_str;
    }

    // Ellipsize: show start and end with ... in middle
    let ellipsis = "...";
    let avail = max_width.saturating_sub(ellipsis.len());
    let start_len = (avail + 1) / 2;
    let end_len = avail / 2;

    let start = &path_str[..start_len.min(path_str.len())];
    let end = if path_str.len() > start_len {
        &path_str[path_str.len() - end_len..]
    } else {
        ""
    };

    format!("{}{}{}", start, ellipsis, end)
}
