use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/package-lock.json");
    println!("cargo:rerun-if-changed=web/vite.config.ts");
    println!("cargo:rerun-if-changed=web/svelte.config.js");
    println!("cargo:rerun-if-changed=web/tailwind.config.js");
    println!("cargo:rerun-if-changed=web/postcss.config.js");
    println!("cargo:rerun-if-changed=web/src/app.html");

    // Always ensure web/build directory exists for include_dir! macro
    let web_build_path = Path::new("web/build");
    if !web_build_path.exists() {
        fs::create_dir_all(web_build_path)
            .expect("Failed to create web/build directory for include_dir! macro");
        println!("Created empty web/build directory for include_dir! macro");
    }

    // Skip web build if environment variable is set
    if std::env::var("SKIP_WEB_BUILD").is_ok() {
        println!("Skipping web build due to SKIP_WEB_BUILD environment variable");
        return;
    }

    // Skip web build in development mode
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile != "release" {
        println!(
            "Skipping web build in development mode (profile: {})",
            profile
        );
        return;
    }

    println!("Building frontend for release...");

    let web_dir = if Path::new("web").exists() {
        "web"
    } else {
        panic!("Could not find web directory");
    };

    // Use npm ci for reproducible builds when lockfile is present
    let lockfile_path = Path::new(web_dir).join("package-lock.json");
    let install_args = if lockfile_path.exists() {
        println!("Running npm ci...");
        vec!["ci"]
    } else {
        println!("Running npm install...");
        vec!["install"]
    };

    let install_output = Command::new("npm")
        .args(&install_args)
        .current_dir(web_dir)
        .output()
        .expect("Failed to execute npm install/ci");

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        panic!(
            "npm {} failed:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
            install_args[0]
        );
    }

    println!("Running npm run build...");
    let output = Command::new("npm")
        .args(["run", "build"])
        .current_dir(web_dir)
        .output()
        .expect("Failed to execute npm run build");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("npm run build failed:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}");
    }
}
