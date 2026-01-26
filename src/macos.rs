use objc::msg_send;
use objc::runtime::Object;
use objc::{class, sel, sel_impl};
use std::ffi::CString;
use std::path::{Path, PathBuf};

pub fn request_folder_access(initial_path: &str) -> Option<PathBuf> {
    unsafe {
        let panel: *mut Object = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panel, setCanChooseFiles:false];
        let _: () = msg_send![panel, setCanChooseDirectories:true];
        let _: () = msg_send![panel, setAllowsMultipleSelection:false];
        
        // Set initial directory
        let path_str = CString::new(initial_path).unwrap();
        let path_obj: *mut Object = msg_send![class!(NSString), stringWithUTF8String: path_str.as_ptr()];
        let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath: path_obj];
        let _: () = msg_send![panel, setDirectoryURL:url];
        
        // Run modal (blocks until user chooses)
        let result: i64 = msg_send![panel, runModal];
        
        if result == 1 { // NSModalResponseOK
            let selected_url: *mut Object = msg_send![panel, URL];
            let path_obj: *mut Object = msg_send![selected_url, path];
            let c_str: *const i8 = msg_send![path_obj, UTF8String];
            let path_str = std::ffi::CStr::from_ptr(c_str).to_string_lossy();
            return Some(PathBuf::from(path_str.to_string()));
        }
    }
    None
}

/// Find images using FileManager.DirectoryEnumerator (handles firmlinks natively)
pub fn find_images(path: &str, max_depth: usize) -> Vec<PathBuf> {
    let image_extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];
    let mut images = Vec::new();
    
    unsafe {
        let fm: *mut Object = msg_send![class!(NSFileManager), defaultManager];
        
        // Convert path to NSURL
        let c_path = CString::new(path).unwrap();
        let path_obj: *mut Object = msg_send![class!(NSString), stringWithUTF8String: c_path.as_ptr()];
        let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath: path_obj];
        
        // Create enumerator - pass nil for properties and error handler
        let nil_ptr: *const std::ffi::c_void = std::ptr::null();
        let enumerator: *mut Object = msg_send![fm, enumeratorAtURL:url includingPropertiesForKeys:nil_ptr options:0 errorHandler:nil_ptr];
        
        if enumerator.is_null() {
            return images;
        }
        
        // Get the base URL's path component count for depth tracking
        let base_components: *mut Object = msg_send![url, pathComponents];
        let base_depth: usize = msg_send![base_components, count];
        
        // Iterate over directory contents
        loop {
            let current_url: *mut Object = msg_send![enumerator, nextObject];
            if current_url.is_null() {
                break;
            }
            
            // Get current URL's depth
            let current_components: *mut Object = msg_send![current_url, pathComponents];
            let current_depth: usize = msg_send![current_components, count];
            let relative_depth = if current_depth >= base_depth {
                current_depth - base_depth
            } else {
                0
            };
            
            // Check if we've exceeded max depth
            if relative_depth > max_depth {
                let _: () = msg_send![enumerator, skipDescendants];
                continue;
            }
            
            // Get path string
            let path_str_obj: *mut Object = msg_send![current_url, path];
            let c_str: *const i8 = msg_send![path_str_obj, UTF8String];
            let path_str = std::ffi::CStr::from_ptr(c_str).to_string_lossy();
            
            // Check if file has image extension
            if let Some(ext) = Path::new(path_str.as_ref()).extension() {
                if let Some(ext_str) = ext.to_str() {
                    if image_extensions.contains(&ext_str.to_lowercase().as_str()) {
                        images.push(PathBuf::from(path_str.to_string()));
                    }
                }
            }
        }
    }
    
    images
}

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
