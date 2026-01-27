# Piccy Picky

## Overview

Piccy Picky is a macOS CLI image triage tool built in Rust. View images from multiple directories in your terminal and quickly decide which to keep or send to trash. Inline display in iTerm2 with interactive k/b/i decisions.

## Features

- **Firmlink-Aware Traversal**: Uses macOS `FileManager.DirectoryEnumerator` to properly handle firmlinks (invisible directory aliases) without duplicate scans
- **Depth-Limited Search**: Recursively search directories up to a specified depth with `-d/--depth` (default: 1)
- **Multi-Directory Support**: Search and triage images from multiple paths in a single session
- **Interactive Workflow**: Quick keys for decisions:
  - **k** - Keep image
  - **b** - Send to Bin/Trash (uses native macOS `trashItemAtURL:` for safe deletion)
  - **i** - Show debug info (terminal size, image metrics, scaling factors)
- **Scaling Modes**: Two display modes (toggle with **m**):
  - **Uniform** (📏) - All images scaled equally to fit
  - **Equal Budget** (🎯) - Each image gets equal row allocation
- **Screen Management**:
  - **Ctrl+L** - Clear screen and redraw undecided images
  - **c** - Continue (pick new batch of 3 images)
  - **r** - Restart (redisplay current 3 images)
  - **q** - Quit
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

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any enhancements or bug fixes.

## License

This project is licensed under the MIT License.