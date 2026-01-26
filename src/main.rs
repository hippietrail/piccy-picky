use rand::seq::SliceRandom;
use std::env;
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
    
    // Try to access the path; if it fails, show permission instructions
    if !Path::new(&target_path).exists() {
        eprintln!("Path does not exist: {}", target_path);
        std::process::exit(1);
    }
    
    if std::fs::read_dir(&target_path).is_err() {
        eprintln!("\n❌ No permission to access: {}", target_path);
        eprintln!("\n📋 To fix this on macOS:");
        eprintln!("   1. System Settings > Privacy & Security > Files and Folders");
        eprintln!("   2. Find iTerm2 and grant it access to this folder");
        eprintln!("\n   Also add iTerm2 to Full Disk Access:");
        eprintln!("   System Settings > Privacy & Security > Full Disk Access");
        std::process::exit(1);
    }

    // Enable raw mode for interactive input
    let original_termios = term::enable_raw_mode()
        .expect("Failed to enable raw mode");
    
    // Ensure we restore on exit
    let _restore = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Deferred cleanup via drop
    }));

    loop {
        // Get terminal size
        let (cols, rows) = term::get_terminal_size();
        let (_, px_height) = term::get_terminal_pixel_size();

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

        // Pre-calculate heights to ensure all 3 fit
        let display_width_chars = 35u32;
        let pixels_per_row = px_height.max(1) / rows.max(1) as u32;
        
        let mut heights: Vec<u32> = Vec::new();
        let mut total_height_rows = 0u32;
        
        for path in &chosen {
            match calc_image_height_rows(path, display_width_chars, pixels_per_row) {
                Ok(h) => {
                    heights.push(h);
                    total_height_rows += h;
                }
                Err(e) => {
                    let abbrev = term::abbreviate_path(path, &target_path, cols as usize);
                    eprintln!("Failed to calc height {}: {}", abbrev, e);
                    total_height_rows = u32::MAX; // Force failure
                }
            }
        }

        // Check if all 3 fit (with 1 row padding between each)
        let padding_rows = (chosen.len() - 1) as u32;
        let total_needed = total_height_rows + padding_rows;
        let available_rows = rows.saturating_sub(5) as u32; // 5 rows reserved for filenames + prompts

        if total_needed > available_rows {
            println!("3 images need {} rows but only {} available. Showing what fits...", total_needed, available_rows);
        }

        // Load and display images (iTerm2 will auto-scale via width parameter)
        let mut displayed = Vec::new();
        for path in &chosen {
            match load_and_display_image(path) {
                Ok(_) => {
                    let abbrev = term::abbreviate_path(path, &target_path, cols as usize);
                    println!("{}", abbrev);
                    displayed.push(path.clone());
                }
                Err(e) => {
                    let abbrev = term::abbreviate_path(path, &target_path, cols as usize);
                    eprintln!("Failed to load {}: {}", abbrev, e);
                }
            }
        }

        if displayed.is_empty() {
            println!("Could not display any images.");
            break;
        }

        // Interactive interface: show [k/b] [k/b] [k/b] with ANSI highlighting
        print!("\n");
        let mut decisions = Vec::new();
        
        for idx in 0..displayed.len() {
            let abbrev = term::abbreviate_path(&displayed[idx], &target_path, cols as usize - 20);
            
            // Build display line with all 3 slots
            let mut line = String::new();
            for i in 0..displayed.len() {
                if i == idx {
                    // Current: bold
                    line.push_str(&format!("\x1b[1m[k/b]\x1b[0m "));
                } else if i < idx {
                    // Done: show what was chosen
                    line.push_str(&format!("[{}]   ", decisions[i]));
                } else {
                    // Upcoming: dim
                    line.push_str("\x1b[2m[k/b]\x1b[0m ");
                }
            }
            line.push_str(&format!("  {}", abbrev));
            
            print!("{}", line);
            io::stdout().flush().unwrap();

            // Read single keypress
            loop {
                if let Ok(c) = term::read_single_char() {
                    match c.to_lowercase().next() {
                        Some('k') => {
                            decisions.push('k');
                            println!();
                            break;
                        }
                        Some('b') => {
                            if macos::move_to_trash(&displayed[idx]) {
                                decisions.push('b');
                                println!();
                                break;
                            } else {
                                eprintln!("\nFailed to bin.");
                                print!("Try again: ");
                                io::stdout().flush().unwrap();
                            }
                        }
                        _ => {
                            print!("\x07"); // Bell
                            io::stdout().flush().unwrap();
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // Ask to continue or quit
        print!("\n[c]ontinue, [q]uit: ");
        io::stdout().flush().unwrap();
        
        loop {
            if let Ok(c) = term::read_single_char() {
                match c.to_lowercase().next() {
                    Some('c') => {
                        println!();
                        break;
                    }
                    Some('q') => {
                        println!();
                        term::disable_raw_mode(&original_termios).ok();
                        std::process::exit(0);
                    }
                    _ => {
                        print!("\x07"); // Bell
                        io::stdout().flush().unwrap();
                    }
                }
            } else {
                break;
            }
        }
    }
    
    // Restore terminal
    let _ = term::disable_raw_mode(&original_termios);
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

/// Pre-calculate image display height in character rows
pub fn calc_image_height_rows(path: &Path, display_width_chars: u32, pixels_per_char: u32) -> Result<u32, String> {
    let img = image::open(path)
        .map_err(|e| e.to_string())?;

    let (w, h) = img.dimensions();
    let aspect_ratio = h as f32 / w as f32;

    // Display width in pixels (35 chars * 8 pixels/char ≈ 280px)
    let display_width_px = display_width_chars * pixels_per_char;

    // Calculate height in pixels using aspect ratio
    let height_px = (display_width_px as f32 * aspect_ratio) as u32;

    // Round UP to nearest character row
    let height_rows = (height_px + pixels_per_char - 1) / pixels_per_char;

    Ok(height_rows)
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
