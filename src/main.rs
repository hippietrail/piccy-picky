use objc::msg_send;
use objc::runtime::Object;
use objc::{class, sel, sel_impl};
use rand::seq::SliceRandom;
use std::env;
use std::ffi::CString;
use std::io::{self, Write, Cursor};
use std::path::{Path, PathBuf};
use image::GenericImageView;

mod macos;
mod term;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: piccy-picky <path>");
        std::process::exit(1);
    }

    let mut target_path = args[1].clone();
    
    // Try to access the path; if it fails, ask user via NSOpenPanel
    if !Path::new(&target_path).exists() || 
       std::fs::read_dir(&target_path).is_err() {
        eprintln!("No access to: {}. Opening folder picker...", target_path);
        if let Some(chosen) = macos::request_folder_access(&target_path) {
            target_path = chosen.to_string_lossy().to_string();
            println!("Selected: {}", target_path);
        } else {
            eprintln!("No folder selected.");
            std::process::exit(1);
        }
    }

    loop {
        // Get terminal size
        let (cols, rows) = term::get_terminal_size();
        println!("Terminal: {}x{}", cols, rows);

        // Walk path (depth 1) and find images
        let images = find_images(&target_path);
        if images.is_empty() {
            println!("No images found in {}", target_path);
            break;
        }

        // Randomly choose 3 (or fewer if not enough)
        let mut rng = rand::thread_rng();
        let chosen: Vec<_> = images
            .choose_multiple(&mut rng, 3.min(images.len()))
            .cloned()
            .collect();

        // Calculate max image height (fit 3 vertically)
        let max_img_height = (rows as u32).saturating_sub(10) / 3;
        let max_img_width = cols as u32;

        // Load, scale, and display images
        let mut displayed = Vec::new();
        for path in &chosen {
            match load_and_display_image(path, max_img_width, max_img_height) {
                Ok(_) => {
                    println!("{}", path.display());
                    displayed.push(path.clone());
                }
                Err(e) => {
                    eprintln!("Failed to load {}: {}", path.display(), e);
                }
            }
        }

        if displayed.is_empty() {
            println!("Could not display any images.");
            break;
        }

        // Interactive menu for each image
        for path in &displayed {
            loop {
                print!(
                    "\n{}\n[k]eep, [b]in, [s]kip: ",
                    path.display()
                );
                io::stdout().flush().unwrap();

                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    break;
                }
                let choice = input.trim().to_lowercase();

                match choice.as_str() {
                    "k" => {
                        println!("Kept.");
                        break;
                    }
                    "b" => {
                        if macos::move_to_trash(path) {
                            println!("Binned.");
                        } else {
                            println!("Failed to bin.");
                        }
                        break;
                    }
                    "s" => {
                        println!("Skipped.");
                        break;
                    }
                    _ if !choice.is_empty() => {
                        println!("Invalid choice. Try again.");
                    }
                    _ => {}
                }
            }
        }

        // Ask user to continue or stop
        print!("\n[c]ontinue, [q]uit: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() == "q" {
            break;
        }
    }
}

fn find_images(path: &str) -> Vec<PathBuf> {
    let mut images = Vec::new();
    let image_extensions = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];

    unsafe {
        let fm: *mut Object = msg_send![class!(NSFileManager), defaultManager];

        let path_ns = CString::new(path).unwrap();
        let path_obj: *mut Object = msg_send![class!(NSString), stringWithUTF8String: path_ns.as_ptr()];

        let enumerator: *mut Object = msg_send![fm, enumeratorAtPath: path_obj];

        if !enumerator.is_null() {
            loop {
                let filename: *mut Object = msg_send![enumerator, nextObject];
                if filename.is_null() {
                    break;
                }

                let c_str: *const i8 = msg_send![filename, UTF8String];
                let filename_str = std::ffi::CStr::from_ptr(c_str).to_string_lossy();

                let full_path = format!("{}/{}", path, filename_str);
                let p = Path::new(&full_path);

                if let Some(ext) = p.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if image_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            images.push(p.to_path_buf());
                        }
                    }
                }
            }
        }
    }

    images
}

fn load_and_display_image(path: &Path, max_width: u32, max_height: u32) -> Result<(), String> {
    let img = image::open(path)
        .map_err(|e| e.to_string())?;

    // Calculate scaling to fit within bounds (don't scale up)
    let (w, h) = img.dimensions();
    let scale = if w > max_width || h > max_height {
        (max_width as f32 / w as f32).min(max_height as f32 / h as f32).min(1.0)
    } else {
        1.0
    };

    let new_w = (w as f32 * scale) as u32;
    let new_h = (h as f32 * scale) as u32;

    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    // Encode to PNG and display
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    resized.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png_data);
    let size = png_data.len();
    println!("\x1b]1337;File=name=image.png;size={};inline=1;width={}px;height={}px;base64:{}\x07", size, new_w, new_h, encoded);
    println!();

    Ok(())
}
