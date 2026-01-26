# Piccy Picky

## Overview

Piccy Picky is a full macOS CLI image viewer built in Rust. It provides an ANSI text-based UI for inline image display in iTerm2, allowing users to view images directly in the terminal. The application uses `NSFileManager` for correct Bin/Trash operations.

## Features

- **Image Viewing**: View images directly in the terminal with inline display.
- **File Management**: Utilizes `NSFileManager` for Bin/Trash operations.
- **Directory Navigation**: Supports recursive directory walking with the `-d/--depth` flag.
- **Mode Toggling**: Toggle display modes using the 'm' key.
- **Debug Information**: Access hidden debug info by pressing the 'i' key.
- **Screen Clearing**: Clear the screen at any time with Ctrl+L.

## Installation

To install Piccy Picky, clone the repository and build the project using Cargo:

```bash
git clone https://github.com/hippietrail/piccy-picky.git
cd piccy-picky
cargo build --release
```

## Usage

Run the application from the terminal:

```bash
./target/release/piccy-picky [options]
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any enhancements or bug fixes.

## License

This project is licensed under the MIT License.