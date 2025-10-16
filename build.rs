use std::process::Command;

fn main() {
    // Set git hash environment variable
    set_git_hash();
}

fn set_git_hash() {
    // Check if we're in a git repository and get the short hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| String::from(""));

    // Check if the current commit has a tag
    let has_tag = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    // Get build date in YYYY-MM-DD format
    let build_date = Command::new("date")
        .args(["+%Y-%m-%d"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| String::from(""));

    // Set the environment variables
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=HAS_GIT_TAG={}", has_tag);
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);

    // Re-run if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
}
