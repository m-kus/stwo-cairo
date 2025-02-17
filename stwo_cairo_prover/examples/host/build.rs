use std::env;

use scarb_metadata::ScarbCommand;

fn main() {
    // Tell Cargo to re-run this if any files in the guest directory change
    println!("cargo:rerun-if-changed=../guest");

    // Run Scarb build
    ScarbCommand::new()
        .arg("build")
        .env("SCARB_PACKAGES_FILTER", "cairo_guest")
        .run()
        .expect("Failed to build");

    // Get the project root directory (two levels up from host)
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let project_root = current_dir
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf();

    // Construct the absolute path to the executable
    let executable_path = project_root
        .join("target")
        .join("dev")
        .join("cairo_guest.executable.json");

    // Tell Cargo where to find the file
    println!(
        "cargo:rustc-env=CAIRO_PROGRAM_PATH={}",
        executable_path.display()
    );
}
