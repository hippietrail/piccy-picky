use objc::msg_send;
use objc::runtime::Object;
use objc::{class, sel, sel_impl};
use std::ffi::CString;
use std::path::Path;

pub fn move_to_trash(path: &Path) -> bool {
    unsafe {
        let fm: *mut Object = msg_send![class!(NSFileManager), defaultManager];

        let path_str = path.to_string_lossy();
        let c_path = CString::new(path_str.as_bytes()).unwrap();
        let path_obj: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: c_path.as_ptr()];

        // Use trashItemAtURL:resultingItemURL:error:
        // This moves to trash without overwriting items with the same name
        let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath: path_obj];

        let mut error: *mut Object = std::ptr::null_mut();
        let result_url: *mut Object = std::ptr::null_mut();
        let success: bool = msg_send![fm, trashItemAtURL:url resultingItemURL:&result_url error:&mut error];

        if !error.is_null() {
            let err_desc: *mut Object = msg_send![error, description];
            let c_str: *const i8 = msg_send![err_desc, UTF8String];
            eprintln!(
                "NSError: {}",
                std::ffi::CStr::from_ptr(c_str).to_string_lossy()
            );
            return false;
        }

        success
    }
}
