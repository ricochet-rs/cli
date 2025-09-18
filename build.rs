use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Try to find ricochet-ui CSS files
    let out_dir = env::var("OUT_DIR").unwrap();

    // Check if ricochet repo exists locally
    let ricochet_ui_path = "../ricochet/ricochet-ui/style";
    if Path::new(ricochet_ui_path).exists() {
        // Copy CSS files to OUT_DIR for inclusion
        let css_files = ["theme.css", "basecoat.css", "tailwind.css"];

        for css_file in &css_files {
            let src = format!("{}/{}", ricochet_ui_path, css_file);
            let dst = format!("{}/{}", out_dir, css_file);

            if Path::new(&src).exists() {
                fs::copy(&src, &dst).unwrap_or_else(|_| {
                    println!("cargo:warning=Could not copy {}", css_file);
                    0
                });
            }
        }

        println!("cargo:rustc-env=RICOCHET_UI_CSS_PATH={}", out_dir);
    } else {
        println!("cargo:warning=ricochet-ui CSS not found, using embedded styles");
    }
}
