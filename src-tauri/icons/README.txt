ICON SETUP
==========

Tauri requires app icons for building. You have two options:

OPTION 1: Generate Icons Automatically (Easiest)
------------------------------------------------
Tauri can generate all required icon sizes from a single PNG:

1. Create or download a 1024x1024 PNG icon (e.g., app-icon.png)
2. Run: cargo tauri icon path/to/app-icon.png
3. This will generate all required sizes in src-tauri/icons/

OPTION 2: Use Default Icons
---------------------------
For development/testing, Tauri CLI will use default icons if none exist.
Just run `cargo tauri dev` or `cargo tauri build` and Tauri will handle it.

REQUIRED ICON SIZES:
-------------------
- 32x32.png
- 128x128.png  
- 128x128@2x.png
- icon.icns (macOS)
- icon.ico (Windows)

NOTE: For the first build, you can skip this entirely - Tauri will use
placeholder icons. Add custom icons later when you're ready to distribute.
