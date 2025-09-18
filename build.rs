use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Try to locate ricochet-ui CSS files from the dependency
    let out_dir = env::var("OUT_DIR").unwrap();

    // Look for ricochet-ui in cargo's git checkouts
    let cargo_home = env::var("CARGO_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap();
        format!("{}/.cargo", home)
    });

    // Try to find the ricochet-ui checkout directory
    let git_checkouts = format!("{}/git/checkouts", cargo_home);

    if let Ok(entries) = fs::read_dir(&git_checkouts) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.to_str().unwrap_or("").contains("ricochet") {
                // Look for style directory in subdirectories
                if let Ok(subdirs) = fs::read_dir(&path) {
                    for subdir in subdirs.flatten() {
                        let style_path = subdir.path().join("ricochet-ui/style");
                        if style_path.exists() {
                            // Found the style directory
                            let theme_css_path = style_path.join("theme.css");
                            let basecoat_css_path = style_path.join("basecoat.css");

                            if theme_css_path.exists() && basecoat_css_path.exists() {
                                // Read and combine CSS files
                                let theme_css = fs::read_to_string(&theme_css_path)
                                    .expect("Failed to read theme.css");
                                let basecoat_css = fs::read_to_string(&basecoat_css_path)
                                    .expect("Failed to read basecoat.css");

                                // Write combined CSS to OUT_DIR
                                let combined_css = format!("{}\n{}", theme_css, basecoat_css);
                                let out_path = Path::new(&out_dir).join("ricochet_ui_styles.css");
                                fs::write(&out_path, combined_css).unwrap();

                                println!(
                                    "cargo:rustc-env=RICOCHET_UI_CSS_PATH={}",
                                    out_path.display()
                                );
                                println!("cargo:rerun-if-changed={}", theme_css_path.display());
                                println!("cargo:rerun-if-changed={}", basecoat_css_path.display());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    panic!("Could not find ricochet-ui CSS files in cargo dependencies");
}
