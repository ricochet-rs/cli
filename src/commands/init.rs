use dialoguer::{Input, Select, theme::ColorfulTheme};
use ricochet_core::{
    content::{AccessType, ContentType},
    language::Language,
};
use std::path::PathBuf;

// Prompts:
//
// Choose a language:
//  - choose a content type from a subset
//  - Give the item a name

pub fn choose_language() -> Language {
    let languages = vec![Language::R, Language::Python, Language::Julia];
    let language_names = vec!["R", "Python", "Julia"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a language")
        .items(&language_names)
        .default(0)
        .interact()
        .unwrap_or(0);

    languages[selection].clone()
}

pub fn choose_content_type(language: &Language) -> ContentType {
    let opts = match language {
        Language::R => {
            vec![
                ContentType::R,
                ContentType::Plumber,
                ContentType::Ambiorix,
                ContentType::Shiny,
                ContentType::Rmd,
                ContentType::RmdShiny,
                ContentType::ServerlessR,
                ContentType::QuartoR,
                ContentType::QuartoRShiny,
            ]
        }
        Language::Julia => {
            vec![
                ContentType::Julia,
                ContentType::JuliaService,
                ContentType::QuartoJl,
            ]
        }
        Language::Python => unimplemented!("Python support is not yet implemented"),
    };

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose content type")
        .items(&opts)
        .default(0)
        .interact()
        .unwrap_or(0);

    opts[selection]
}

fn choose_item_name() -> String {
    use dialoguer::Input;

    Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Content item name")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                Err("Name cannot be empty")
            } else if input.len() > 120 {
                Err("Name must be 120 characters or less")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .unwrap_or_default()
}

/// Find files with the given extension in the specified directory and one level deep
fn find_files_by_extension(extension: &str, search_dir: &PathBuf) -> Vec<PathBuf> {
    use std::fs;

    let mut files = Vec::new();

    // Search in specified directory
    if let Ok(entries) = fs::read_dir(search_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Add files with matching extension in current directory
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == extension {
                        if let Ok(relative) = path.strip_prefix(search_dir) {
                            files.push(relative.to_path_buf());
                        }
                    }
                }
            }

            // Search one level deep in non-hidden directories
            if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    if !dir_name.to_string_lossy().starts_with('.') {
                        if let Ok(sub_entries) = fs::read_dir(&path) {
                            for sub_entry in sub_entries.filter_map(|e| e.ok()) {
                                let sub_path = sub_entry.path();
                                if sub_path.is_file() {
                                    if let Some(ext) = sub_path.extension() {
                                        if ext == extension {
                                            if let Ok(relative) = sub_path.strip_prefix(search_dir)
                                            {
                                                files.push(relative.to_path_buf());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    files.sort();
    files
}

fn custom_entrypoint(prompt: &str, extension: &str) -> anyhow::Result<PathBuf> {
    let error_msg = format!("Path must end with {}", extension);

    let path = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .validate_with(move |input: &String| -> Result<(), String> {
            if input.trim().is_empty() {
                Err("Path cannot be empty".to_string())
            } else if !input.ends_with(extension) {
                Err(error_msg.clone())
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    Ok(PathBuf::from(path))
}

fn choose_r_entrypoint(dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let r_files = find_files_by_extension("R", dir);

    if r_files.is_empty() {
        println!("No .R files found in {}", dir.display());
        custom_entrypoint("Enter path to R entrypoint", ".R")
    } else {
        let display_items: Vec<String> = r_files.iter().map(|p| p.display().to_string()).collect();

        let selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose R entrypoint")
            .items(&display_items)
            .default(0)
            .interact()?;
        Ok(r_files[selection].clone())
    }
}

fn choose_entrypoint(content_type: &ContentType, dir: &PathBuf) -> anyhow::Result<PathBuf> {
    match content_type {
        ContentType::R
        | ContentType::Plumber
        | ContentType::RService
        | ContentType::ServerlessR => choose_r_entrypoint(dir),
        ContentType::Ambiorix => todo!(),
        ContentType::Shiny => todo!(),
        ContentType::Rmd => todo!(),
        ContentType::RmdShiny => todo!(),
        ContentType::Julia => todo!(),
        ContentType::JuliaService => todo!(),
        ContentType::QuartoR => todo!(),
        ContentType::QuartoRShiny => todo!(),
        ContentType::QuartoJl => todo!(),
        ContentType::ServerlessJl => todo!(),
        ContentType::Python => todo!(),
        ContentType::PythonService => todo!(),
    }
}

fn choose_access_type() -> AccessType {
    let opts = [
        AccessType::Private,
        AccessType::Internal,
        AccessType::External,
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Item visibility")
        .items(&opts)
        .default(0)
        .interact()
        .unwrap_or(0);

    opts[selection].clone()
}

pub fn init_rico_toml(dir: &PathBuf) -> anyhow::Result<()> {
    let lang = choose_language();
    let content_type = choose_content_type(&lang);
    let entrypoint = choose_entrypoint(&content_type, dir)?;
    let content_name = choose_item_name();
    let at = choose_access_type();

    println!("Language: {lang}\nItem type: {content_type}\nName: {content_name}");
    Ok(())
}
