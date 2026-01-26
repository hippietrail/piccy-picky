use libc::{ioctl, isatty, STDOUT_FILENO, TIOCGWINSZ};

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
