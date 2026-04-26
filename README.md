# rbxlx-to-rojo
Forked converter for turning Roblox place/model files into Rojo-friendly project structures.

## Credits
- Original project: rojo-rbx/rbxl-to-rojo
- Original author: Kampfkarren
- This fork keeps the original MPL-2.0 license and attribution.

## What This Fork Adds
- Keeps binary input support for .rbxl and .rbxm.
- Detects and decodes XML format files by file content.
- Preserves full recursive hierarchy under StarterGui (children, grandchildren, and deeper).
- Exports full properties for StarterGui descendants into generated meta output.

## Usage
### Setup
Before using this tool, make sure you have:
- A place/model file exported from Roblox Studio.
- Scripts in the source data if you rely on script-driven extraction outside StarterGui.

### Build
Build the CLI binary from the repository root:

cargo build --release --features cli --bin rbxlx-to-rojo

The generated executable is:

target/release/rbxlx-to-rojo.exe

### Run
Launch the executable and choose:
1. Input place/model file.
2. Output folder.

If successful, the output folder will include source files and a default.project.json structure for Rojo.

## License
This project is available under the Mozilla Public License, Version 2.0.
See [LICENSE.md](LICENSE.md).
