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

        // Load and display images (iTerm2 will auto-scale via width parameter)
        let mut displayed = Vec::new();
        for path in &chosen {
            match load_and_display_image(path) {
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

    // Use std fs to enumerate only depth-1 (direct children only)
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                // Skip directories
                if metadata.is_dir() {
                    continue;
                }

                let path_buf = entry.path();
                if let Some(ext) = path_buf.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if image_extensions.contains(&ext_str.to_lowercase().as_str()) {
                            images.push(path_buf);
                        }
                    }
                }
            }
        }
    }

    images
}

fn load_and_display_image(path: &Path) -> Result<(), String> {
    let img = image::open(path)
        .map_err(|e| e.to_string())?;

    // Only scale down if image is extremely large (to avoid huge base64)
    let (w, h) = img.dimensions();
    let max_dim = 2000u32;
    let scale = if w > max_dim || h > max_dim {
        (max_dim as f32 / w.max(h) as f32).min(1.0)
    } else {
        1.0
    };

    let img_to_encode = if scale < 1.0 {
        let new_w = (w as f32 * scale) as u32;
        let new_h = (h as f32 * scale) as u32;
        img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Encode to PNG and display
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    img_to_encode.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png_data);
    let size = encoded.len();
    // Display at ~35 character width (lets iTerm2 auto-scale height preserving aspect ratio)
    println!("\x1b]1337;File=name=image.png;size={};inline=1;width=35c;base64:{}\x07", size, encoded);

    Ok(())
}
