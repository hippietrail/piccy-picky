# Piccy Picky

## Overview

Piccy Picky is a macOS CLI image triage tool built in Rust. View 3 random images at a time in your terminal and quickly decide which to keep or send to trash. Inline display in iTerm2 with interactive k/b/i decisions and intelligent automatic scaling.

## Features

- **Firmlink-Aware Traversal**: Uses macOS `FileManager.DirectoryEnumerator` to properly handle firmlinks (invisible directory aliases) without duplicate scans
- **Depth-Limited Search**: Recursively search directories up to a specified depth with `-d/--depth` (default: 1)
- **Multi-Directory Support**: Search and triage images from multiple paths in a single session
- **Interactive Workflow**: Quick keys for decisions:
  - **k** - Keep image (move to next batch)
  - **b** - Send to Bin/Trash (uses native macOS `trashItemAtURL:` for safe deletion)
  - **i** - Show current image info (dimensions, scaling)
  - **I** - Show comprehensive scaling info for all 3 images + space calculations
  - **Space/L** - Open QuickLook preview
  - **q** - Quit
- **Smart Scaling**: 
  - Automatically detects terminal dimensions (character grid and pixel size)
  - Calculates optimal scale factor to fit all 3 images without overflow
  - Single-pass scaling via iTerm2 (no double-scaling)
  - 2% safety buffer for rounding precision
- **After Batch**:
  - **c** - Continue (pick new batch of 3 images)
  - **r** - Restart (redisplay current 3 images)
  - **q** - Quit
- **Screen Management**:
  - **Ctrl+L** - Clear screen and redraw remaining undecided images
- **System-Aware**: Automatically skips `.Trash`, `.Volumes`, `.TemporaryItems`, `.DS_Store`
- **Test Mode**: `--test-search` flag to preview found images without interactive UI

## Installation

```bash
git clone https://github.com/hippietrail/piccy-picky.git
cd piccy-picky
cargo build --release
```

## Usage

```bash
# Single directory
./target/release/piccy-picky ~/Pictures

# Multiple directories with depth limit
./target/release/piccy-picky -d 2 ~/Pictures ~/Desktop ~/.downloads

# Test search (preview images, no UI)
./target/release/piccy-picky --test-search ~/Pictures -d 2
```

### Options

- `-d, --depth <N>` - Search depth (default: 1). Use 0 for single level only.
- `--test-search` - Test image discovery and exit (shows first 10 matches)
- Multiple paths supported - triage images from multiple directories

## How Scaling Works

Piccy Picky uses iTerm2's inline image protocol to display images efficiently:

1. **Terminal Detection**: Gets both character grid size (cols×rows) and pixel dimensions
2. **Per-Image Width Calculation**: Each image is scaled to fit the available terminal width
3. **Global Scale Factor**: If all 3 images exceed available height, a uniform scale factor is applied to all
4. **iTerm2 Rendering**: Images are displayed using the width parameter (in character cells), letting iTerm2 handle final scaling while preserving aspect ratio
5. **Safety Buffer**: 2% buffer added for rounding precision when converting pixels to character rows

Press [I] to see detailed calculations for the current batch.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any enhancements or bug fixes.

## License

This project is licensed under the MIT License.
