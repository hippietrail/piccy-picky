#![allow(unexpected_cfgs)]

use rand::seq::SliceRandom;
use std::env;
use std::io::{self, Write, Cursor};
use std::path::{Path, PathBuf};
use image::GenericImageView;

mod macos;
mod term;

// Single scaling algorithm implemented:
// 1. Fit each image to available width (in pixels)
// 2. If all 3 heights exceed available height, scale all down uniformly
// Uniform scaling ensures all images scale proportionally together

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: piccy-picky [OPTIONS] <path> [path2] ...");
        eprintln!("Options:");
        eprintln!("  -d, --depth <N>      Search depth (default: 1)");
        eprintln!("  --test-search        Test file search only (print results and exit)");
        std::process::exit(1);
    }

    // Parse CLI args
    let mut target_paths = Vec::new();
    let mut depth = 1usize;
    let mut test_search = false;
    let mut i = 1;
    
    while i < args.len() {
        match args[i].as_str() {
            "-d" | "--depth" => {
                i += 1;
                if i < args.len() {
                    depth = args[i].parse().unwrap_or(1);
                }
            }
            "--test-search" => {
                test_search = true;
            }
            arg if !arg.starts_with('-') => {
                target_paths.push(arg.to_string());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }
    
    if target_paths.is_empty() {
        eprintln!("Error: at least one path required");
        std::process::exit(1);
    }
    
    // If test mode, just search and print results
    if test_search {
        let mut all_images = Vec::new();
        for path in &target_paths {
            let images = macos::find_images(path, depth);
            all_images.extend(images);
        }
        println!("Found {} image files:", all_images.len());
        for (idx, img) in all_images.iter().take(10).enumerate() {
            println!("  {}. {}", idx + 1, img.display());
        }
        if all_images.len() > 10 {
            println!("  ... and {} more", all_images.len() - 10);
        }
        std::process::exit(0);
    }
    
    // Verify all paths exist and are accessible
    for target_path in &target_paths {
        if !Path::new(target_path).exists() {
            eprintln!("Path does not exist: {}", target_path);
            std::process::exit(1);
        }
        
        if std::fs::read_dir(target_path).is_err() {
            eprintln!("\n❌ No permission to access: {}", target_path);
            eprintln!("\n📋 To fix this on macOS:");
            eprintln!("   1. System Settings > Privacy & Security > Files and Folders");
            eprintln!("   2. Find iTerm2 and grant it access to this folder");
            eprintln!("\n   Also add iTerm2 to Full Disk Access:");
            eprintln!("   System Settings > Privacy & Security > Full Disk Access");
            std::process::exit(1);
        }
        }

        // Enable raw mode for interactive input
        let original_termios = term::enable_raw_mode()
        .expect("Failed to enable raw mode");
        
        // Ensure we restore on exit
        let _restore = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Deferred cleanup via drop
        }));

        let mut chosen: Option<Vec<PathBuf>> = None;

        // Scan all images once at the start
        let mut images = Vec::new();
        for path in &target_paths {
            let path_images = macos::find_images(path, depth);
            images.extend(path_images);
        }
        if images.is_empty() {
            println!("No images found in paths: {}", target_paths.join(", "));
            std::process::exit(0);
        }

        loop {
        // Get terminal dimensions
        // CRITICAL: These are our single source of truth for layout calculations.
        // We work primarily in pixels for precision, then convert to character dimensions only for iTerm2.
        let (cols, rows) = term::get_terminal_size();           // Character grid dimensions
        let (px_width, px_height) = term::get_terminal_pixel_size(); // Pixel dimensions of terminal

        // Check if we've run out of images
        if images.is_empty() {
            println!("\n✨ All images reviewed! No more to pick from.");
            break;
        }

        // Pick 3 new images
        if chosen.is_none() {
            let mut rng = rand::thread_rng();
            let batch_size = 3.min(images.len());
            chosen = Some(
                images
                    .choose_multiple(&mut rng, batch_size)
                    .cloned()
                    .collect()
            );
        }
        
        let chosen_ref = chosen.as_ref().unwrap();
        
        // ===== SCALING ALGORITHM =====
        // Goal: Fit 3 images in available space without double-scaling
        //
        // Step 1: Calculate available space in PIXELS (not characters)
        //   - UI needs ~5 rows = 5 * (px_height/rows)
        //   - Available height in pixels = px_height - ui_rows_px
        //   - Available width in pixels = responsive to terminal width, not hardcoded to 35 chars
        //
        // Step 2: For each image, calculate scaled dimensions
        //   - We calculate what pixel width it should be: responsive to terminal width
        //   - Use aspect ratio to get corresponding height in pixels
        //   - NO pre-scaling of images during encoding (except for massive images >4000px)
        //
        // Step 3: Check if 3 scaled images fit vertically
        //   - Sum pixel heights of 3 images + padding
        //   - If over budget: calculate uniform scale-down factor (applies to all 3 equally)
        //
        // Step 4: Pass final pixel dimensions to load_and_display_image()
        //   - Only apply scale during encoding if needed for size
        //   - Let iTerm2 do the final scaling via width parameter
        //
        // KEY: Never scale twice. Our calculations tell iTerm2 exactly what to display.
        
        // Available space in PIXELS
        let ui_rows = 5u32;
        let ui_height_px = ui_rows * (px_height / rows.max(1) as u32);
        let available_height_px = px_height.saturating_sub(ui_height_px);
        
        // Available width: responsive to terminal, with margin for safety
        let width_margin_cols = 2u32;
        let available_width_cols = cols.saturating_sub(width_margin_cols as u16) as u32;
        let available_width_px = available_width_cols * (px_width / cols.max(1) as u32);
        
        // Use the full available width for display, not hardcoded 35 chars
         let display_width_chars = available_width_cols;
         let pixels_per_char_h = px_height.max(1) / rows.max(1) as u32;
         let pixels_per_char_w = px_width.max(1) / cols.max(1) as u32;
         let available_rows = rows.saturating_sub(5) as u32; // 5 rows reserved
         
         // STEP 1: Calculate scale factor needed to fit all 3 images vertically
         // For each image: given display_width_chars and its aspect ratio, what height does it need?
         // If sum of heights > available height, scale down all 3 uniformly
         let mut scale_factor = 1.0f32;
         let mut total_height_rows = 0u32;
         
         for path in chosen_ref {
             match calc_image_height_rows(path, display_width_chars, pixels_per_char_w, pixels_per_char_h) {
                 Ok(h) => {
                     total_height_rows += h;
                 }
                 Err(e) => {
                     let abbrev = term::abbreviate_path(path, "", cols as usize);
                     eprintln!("Failed to calc height {}: {}", abbrev, e);
                 }
             }
         }
         
         // If total height exceeds available, calculate uniform scale-down
         // Add 2% safety buffer for rounding errors (ceil when converting px to rows)
         if total_height_rows > available_rows {
             scale_factor = (available_rows as f32 / total_height_rows as f32) * 0.98;
         }

         // Load and display images
          // Scale the display width by our layout_scale factor, then let iTerm2 handle all rendering
          // This avoids double-scaling: we reduce the width budget, iTerm2 scales image to fit
          let scaled_display_width_chars = ((display_width_chars as f32) * scale_factor) as u32;
         let mut displayed: Vec<(PathBuf, ImageInfo)> = Vec::new();
         for path in chosen_ref {
             match load_and_display_image(path, scaled_display_width_chars) {
                Ok(info) => {
                    let abbrev = term::abbreviate_path(path, "", cols as usize);
                    println!("{}", abbrev);
                    displayed.push((path.clone(), info));
                }
                Err(e) => {
                    let abbrev = term::abbreviate_path(path, "", cols as usize);
                    eprintln!("Failed to load {}: {}", abbrev, e);
                }
            }
        }

        if displayed.is_empty() {
            println!("Could not display any images.");
            break;
        }

        // Show count before prompts
        println!("📸 Picked {} images out of {}", chosen_ref.len(), images.len());

        // Interactive interface: show [k/b/i] [k/b/i] [k/b/i] with ANSI highlighting
         let mut decisions = Vec::new();
        
        for idx in 0..displayed.len() {
            let (path, info) = &displayed[idx];
            let abbrev = term::abbreviate_path(path, "", cols as usize - 20);
            
            loop {
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
                
                print!("\r\x1b[K{}", line); // \r = carriage return, \x1b[K = clear to end of line
                io::stdout().flush().unwrap();

                // Read single keypress
                if let Ok(c) = term::read_single_char() {
                    let code = c as u32;
                    
                    // Ctrl+L = clear screen and redraw undecided images
                    if code == 12 {
                        println!("\x1b[2J\x1b[H"); // Clear screen and move cursor home
                        
                        // Redraw images not yet decided (idx..displayed.len())
                        for i in idx..displayed.len() {
                            let (path, _) = &displayed[i];
                            match load_and_display_image(path, scaled_display_width_chars) {
                                Ok(_) => {
                                    let abbrev = term::abbreviate_path(path, "", cols as usize);
                                    println!("{}", abbrev);
                                }
                                Err(_) => {} // Silently skip redraw errors
                            }
                        }
                        
                        // Redraw image count and continue with current prompt
                        println!("\n📸 Picked {} images out of {}", displayed.len(), images.len());
                        continue; // Skip to next iteration of inner prompt loop
                    }
                    
                    // Check original char BEFORE lowercasing so we can distinguish 'i' vs 'I'
                    match c {
                        'I' => {
                            // Capital [I]: show comprehensive info for all 3 images + calculations
                            display_full_scaling_info(&displayed, cols, rows, px_width, px_height, 
                                                     scale_factor, available_height_px, available_width_px);
                            // Wait for keypress
                            let _ = term::read_single_char();
                            println!("\n");
                            continue;
                        }
                        'i' => {
                            // Lowercase [i]: show info for current image only
                            println!("\n\n📊 Image Info (current):");
                            println!("  Terminal:           {} cols × {} rows", cols, rows);
                            println!("  Terminal pixels:    {} × {} px", px_width, px_height);
                            let px_per_char_h = px_height / rows as u32;
                            let px_per_char_w = px_width / cols as u32;
                            println!("  Pixel per char:     {} × {} px/char", px_per_char_w, px_per_char_h);
                            println!("  Original image:     {} × {} px", info.orig_w, info.orig_h);
                            println!("  Scaling factor:     {:.2}", info.scale_factor);
                            println!("  Scaled image:       {} × {} px", info.scaled_w, info.scaled_h);
                            println!("  Display in term:    35 chars × ~{} chars", 
                                     (info.scaled_h + px_per_char_h - 1) / px_per_char_h);
                            println!("  (press any key to continue)");
                            io::stdout().flush().unwrap();
                            
                            // Wait for keypress
                            let _ = term::read_single_char();
                            println!("\n"); // Clear and restart
                            continue;
                        }
                        _ => {
                            // Lowercase other keys for case-insensitive matching
                            match c.to_lowercase().next() {
                                Some('k') => {
                                    decisions.push('k');
                                    // Remove from collection
                                    images.retain(|p| p != path);
                                    break;
                                }
                                Some('b') => {
                                    if macos::move_to_trash(path) {
                                        decisions.push('b');
                                        // Remove from collection
                                        images.retain(|p| p != path);
                                        break;
                                    } else {
                                        print!("\x07"); // Bell on failure
                                        io::stdout().flush().unwrap();
                                    }
                                }
                                Some(' ') | Some('l') => {
                                    // Open QuickLook preview (hidden, no prompt)
                                    macos::quicklook_preview(path);
                                    continue;
                                }
                                Some('q') => {
                                    // Quit (hidden)
                                    term::disable_raw_mode(&original_termios).ok();
                                    std::process::exit(0);
                                }
                                _ => {
                                    print!("\x07"); // Bell on invalid input
                                    io::stdout().flush().unwrap();
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // All decisions made, move to next line and ask to continue
        println!("\n[c]ontinue, [r]estart, [q]uit: ");
        io::stdout().flush().unwrap();
        
        loop {
            if let Ok(c) = term::read_single_char() {
                match c.to_lowercase().next() {
                    Some('c') => {
                        println!();
                        chosen = None; // Pick new 3 images
                        break;
                    }
                    Some('r') => {
                        println!("\x1b[2J\x1b[H"); // Clear screen and restart loop
                        chosen = None; // Pick new 3 images
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



/// Pre-calculate image display height in character rows
pub fn calc_image_height_rows(path: &Path, display_width_chars: u32, pixels_per_char_w: u32, pixels_per_char_h: u32) -> Result<u32, String> {
    let img = image::open(path)
        .map_err(|e| e.to_string())?;

    let (w, h) = img.dimensions();
    let aspect_ratio = h as f32 / w as f32;

    // Display width in pixels (35 chars * pixels_per_char_w)
    let display_width_px = display_width_chars * pixels_per_char_w;

    // Calculate height in pixels using aspect ratio
    let height_px = (display_width_px as f32 * aspect_ratio) as u32;

    // Round UP to nearest character row
    let height_rows = (height_px + pixels_per_char_h - 1) / pixels_per_char_h;

    Ok(height_rows)
}

pub struct ImageInfo {
    pub orig_w: u32,
    pub orig_h: u32,
    pub scaled_w: u32,
    pub scaled_h: u32,
    pub scale_factor: f32,
}

/// Display comprehensive scaling info for all 3 images + calculations
/// Shows original sizes, available space, scale factors, and final display dimensions
fn display_full_scaling_info(
    displayed: &[(PathBuf, ImageInfo)],
    cols: u16,
    rows: u16,
    px_width: u32,
    px_height: u32,
    scale_factor: f32,
    available_height_px: u32,
    available_width_px: u32,
) {
    println!("\n\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║                    COMPREHENSIVE SCALING INFO [I]                    ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    
    // Terminal info
    println!("\n📱 TERMINAL:");
    println!("  Character grid:     {} cols × {} rows", cols, rows);
    println!("  Pixel dimensions:   {} × {} px", px_width, px_height);
    let px_per_char_w = px_width / cols.max(1) as u32;
    let px_per_char_h = px_height / rows.max(1) as u32;
    println!("  Pixels per char:    {} × {} px/char (w × h)", px_per_char_w, px_per_char_h);
    
    // Available space
    println!("\n📏 AVAILABLE SPACE:");
    let ui_rows = 5u32;
    let ui_height_px = ui_rows * px_per_char_h;
    println!("  UI height:          {} rows = {} px", ui_rows, ui_height_px);
    println!("  Available height:   {} px ({} rows)", available_height_px, 
             available_height_px / px_per_char_h.max(1));
    let width_margin_cols = 2u32;
    let available_width_cols = (cols as u32).saturating_sub(width_margin_cols);
    println!("  Available width:    {} px ({} cols, margin {})", 
             available_width_px, available_width_cols, width_margin_cols);
    
    // Per-image breakdown
    println!("\n🖼️  IMAGES (3 shown):");
    println!("  Global scale factor: {:.3}", scale_factor);
    
    for (idx, (path, info)) in displayed.iter().enumerate() {
        let abbrev = term::abbreviate_path(path, "", 50);
        println!("\n  Image {}:", idx + 1);
        println!("    File:             {}", abbrev);
        println!("    Original:         {} × {} px", info.orig_w, info.orig_h);
        println!("    Original aspect:  {:.3}:1", info.orig_h as f32 / info.orig_w as f32);
        println!("    After scaling:    {} × {} px", info.scaled_w, info.scaled_h);
        println!("    Scaling applied:  {:.3}× (multiply original by this)", info.scale_factor);
        
        // Calculate what this image would be at the available width
        let theoretical_w = available_width_px;
        let theoretical_h = (theoretical_w as f32 * info.orig_h as f32 / info.orig_w as f32) as u32;
        println!("    If scaled to available width: {} × {} px", theoretical_w, theoretical_h);
        
        // Display dimensions accounting for global scale factor
        // When we pass scaled_display_width_chars to iTerm2, it scales height proportionally
        let scaled_display_h = (theoretical_h as f32 * scale_factor) as u32;
        let display_rows = (scaled_display_h + px_per_char_h - 1) / px_per_char_h;
        println!("    Actual display (after scale): {} × {} px = ~{} chars tall", theoretical_w, scaled_display_h, display_rows);
    }
    
    // Summary validation
    println!("\n✅ VALIDATION:");
    // Calculate actual display heights (after applying global scale factor)
    let mut total_actual_height_px = 0u32;
    for (_, info) in displayed.iter() {
        let theoretical_h = (available_width_px as f32 * info.orig_h as f32 / info.orig_w as f32) as u32;
        let scaled_h = (theoretical_h as f32 * scale_factor) as u32;
        total_actual_height_px += scaled_h;
    }
    let total_needed_px = total_actual_height_px;
    
    println!("  Total image heights (after scale): {} px", total_actual_height_px);
    println!("  Total needed:        {} px", total_needed_px);
    println!("  Available:           {} px", available_height_px);
    
    if total_needed_px <= available_height_px {
        println!("  ✓ FITS with {} px to spare ({:.1}% utilized)", 
                 available_height_px - total_needed_px,
                 (total_needed_px as f32 / available_height_px as f32) * 100.0);
    } else {
        let overage = total_needed_px - available_height_px;
        println!("  ✗ OVERFLOW by {} px ({:.1}% over budget)", 
                 overage,
                 (overage as f32 / available_height_px as f32) * 100.0);
    }
    
    println!("\n  (press any key to continue)");
    io::stdout().flush().unwrap();
}

fn load_and_display_image(path: &Path, display_width_chars: u32) -> Result<ImageInfo, String> {
    // CRITICAL: Never scale twice. 
    // display_width_chars is ALREADY scaled by layout_scale (done in main loop).
    // We now just load the image and tell iTerm2 what width to display it at.
    // iTerm2 handles all the scaling to fit that width while preserving aspect ratio.
    //
    // Flow:
    // 1. Load image at original size (reduce only if >4000px for file size)
    // 2. Encode to PNG
    // 3. Tell iTerm2 the display_width_chars (already scaled down if needed)
    // 4. iTerm2 scales image to fit that width, maintaining aspect ratio
    // Result: single scaling pass, no overflow
    
    let img = image::open(path)
        .map_err(|e| e.to_string())?;

    let (w, h) = img.dimensions();
    
    // Only apply encode_scale for truly massive images (>4000px) to reduce file size
    // Do NOT apply layout_scale to the image—let iTerm2 handle that via the width parameter
    let max_dim = 4000u32;
    let encode_scale = if w > max_dim || h > max_dim {
        (max_dim as f32 / w.max(h) as f32).min(1.0)
    } else {
        1.0
    };

    let final_w = (w as f32 * encode_scale) as u32;
    let final_h = (h as f32 * encode_scale) as u32;

    let img_to_encode = if encode_scale < 1.0 {
        let scaled_w = (w as f32 * encode_scale) as u32;
        let scaled_h = (h as f32 * encode_scale) as u32;
        img.resize_exact(scaled_w, scaled_h, image::imageops::FilterType::Lanczos3)
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
    
    // Pass the display_width to iTerm2 - this tells it how wide to make the image
    // iTerm2 will scale the image to fit this width and maintain aspect ratio
    println!("\x1b]1337;File=name=image.png;size={};inline=1;width={}c;base64:{}\x07", 
             size, display_width_chars, encoded);

    Ok(ImageInfo {
        orig_w: w,
        orig_h: h,
        scaled_w: final_w,
        scaled_h: final_h,
        scale_factor: encode_scale,  // Only encode_scale, not layout_scale (iTerm2 handles that)
    })
}
