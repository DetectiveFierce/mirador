use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Path to the source icon
    let icon_path = Path::new(&manifest_dir)
        .join("assets")
        .join("maze-icon.ico");

    if !icon_path.exists() {
        panic!("Icon file not found: {:?}", icon_path);
    }

    // Tell Cargo to rerun this build script if the icon changes
    println!("cargo:rerun-if-changed=assets/maze-icon.ico");

    // Windows: Use winres to embed the icon
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon(&icon_path.to_string_lossy());
        res.compile().expect("Failed to compile Windows resources");
    }

    // Linux: Create a desktop file and install script
    #[cfg(target_os = "linux")]
    {
        // Copy the ICO file to output directory
        let ico_path = Path::new(&out_dir).join("mirador-icon.ico");
        std::fs::copy(&icon_path, &ico_path).expect("Failed to copy ICO icon");

        // Create desktop file content
        let desktop_content = "[Desktop Entry]\n\
Version=1.0\n\
Type=Application\n\
Name=Mirador\n\
Comment=Maze exploration game\n\
Exec=/usr/local/bin/mirador\n\
Icon=mirador\n\
Terminal=false\n\
Categories=Game;";

        // Write desktop file
        let desktop_path = Path::new(&out_dir).join("mirador.desktop");
        std::fs::write(&desktop_path, desktop_content).expect("Failed to write desktop file");

        // Create installation script
        let install_script = format!(
            r#"#!/bin/bash
# Auto-generated installation script for Mirador

set -e

EXECUTABLE="{}/target/release/mirador"
DESKTOP_FILE="{}"
ICON_FILE="{}"

if [ ! -f "$EXECUTABLE" ]; then
    echo "Error: Executable not found. Please run 'cargo build --release' first."
    exit 1
fi

# Create directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/share/applications
sudo mkdir -p /usr/share/icons/hicolor/256x256/apps

# Install files
sudo cp "$EXECUTABLE" /usr/local/bin/
sudo cp "$DESKTOP_FILE" /usr/share/applications/
sudo cp "$ICON_FILE" /usr/share/icons/hicolor/256x256/apps/mirador.ico

# Update icon cache
sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor

echo "Mirador installed successfully!"
echo "You can now find it in your application menu."#,
            manifest_dir,
            desktop_path.to_string_lossy(),
            ico_path.to_string_lossy()
        );

        let install_path = Path::new(&out_dir).join("install.sh");
        std::fs::write(&install_path, install_script).expect("Failed to write install script");

        // Make install script executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_path, perms)
            .expect("Failed to set executable permissions");

        println!(
            "cargo:rustc-env=LINUX_INSTALL_SCRIPT={}",
            install_path.to_string_lossy()
        );
        println!(
            "cargo:rustc-env=LINUX_DESKTOP_FILE={}",
            desktop_path.to_string_lossy()
        );
    }
}
