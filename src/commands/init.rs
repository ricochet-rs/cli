use anyhow::bail;
use dialoguer::{Confirm, FuzzySelect, Input, Select, theme::ColorfulTheme};
use ricochet_core::{
    content::{AccessType, Content, ContentItem, ContentType},
    language::{Language, LanguageConfig, Package},
    settings::{ScheduleSettings, ServeSettings, StaticSettings},
};
use std::path::PathBuf;
use walkdir::WalkDir;

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

pub fn choose_content_type(language: &Language) -> anyhow::Result<ContentType> {
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
        Language::Python => {
            bail!("Python is not yet implemented")
        }
    };

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose content type")
        .items(&opts)
        .default(0)
        .interact()
        .unwrap_or(0);

    Ok(opts[selection])
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

/// FIXME: replace with WalkDir:
/// https://docs.rs/walkdir/latest/walkdir/struct.WalkDir.html#method.max_depth
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

fn choose_r_entrypoint(dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let r_files = find_files_by_extension("R", dir);

    if r_files.is_empty() {
        bail!("No .R files found in {}", dir.display());
    } else {
        let display_items: Vec<String> = r_files.iter().map(|p| p.display().to_string()).collect();

        let selection = dialoguer::FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose R entrypoint")
            .highlight_matches(true)
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

fn static_settings(
    path: &PathBuf,
    content_type: &ContentType,
) -> anyhow::Result<Option<StaticSettings>> {
    if !content_type.maybe_static() {
        return Ok(None);
    }

    let theme = ColorfulTheme::default();

    // if they skip non static html
    let Some(opt) = Confirm::with_theme(&theme)
        .with_prompt("Serve this item as a static HTML site?")
        .interact_opt()?
    else {
        return Ok(None);
    };

    // if they do not confirm then no static html
    if !opt {
        return Ok(None);
    };

    let mut static_settings = StaticSettings::default();

    let dirs = WalkDir::new(path)
        .max_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter(|v| v.as_ref().is_ok_and(|vv| vv.file_type().is_dir()))
        .filter_map(|vi| vi.ok().map(|ii| ii.file_name().display().to_string()))
        .collect::<Vec<_>>();

    let Some(opt) = FuzzySelect::with_theme(&theme)
        .with_prompt("Which directory should be served?")
        .items(&dirs)
        .highlight_matches(true)
        .interact_opt()?
    else {
        return Ok(None);
    };

    let serve_dir = dirs[opt].to_string();

    let entrypoint = Input::with_theme(&theme)
        .with_prompt("Which file should be served?")
        .default("index.html".to_string())
        .show_default(true)
        .with_initial_text("index.html")
        .interact_text()?;
    static_settings.index = Some(entrypoint);
    static_settings.output_dir = Some(serve_dir);

    Ok(Some(static_settings))
}

fn schedule(content_type: &ContentType) -> anyhow::Result<Option<ScheduleSettings>> {
    if !content_type.is_invokable() {
        return Ok(None);
    }
    let theme = ColorfulTheme::default();

    // if they skip non static html
    let Some(opt) = Confirm::with_theme(&theme)
        .with_prompt("Schedule this item?")
        .interact_opt()?
    else {
        return Ok(None);
    };

    // if they do not confirm then no static html
    if !opt {
        return Ok(None);
    };

    let mut sched = ScheduleSettings::default();
    let opts = ["@hourly", "@daily", "@weekly", "Custom (enter cron)"];
    let opt = FuzzySelect::with_theme(&theme)
        .with_prompt("Schedule item")
        .items(opts)
        .default(0)
        .interact()?;

    if opt.eq(&0usize) {
        return Ok(None);
    }

    if opt.eq(&3usize) {
        let cron = Input::with_theme(&theme)
            .with_prompt("Enter cron schedule")
            .with_initial_text("0 0 * * *")
            .validate_with(|v: &String| {
                let mut sched = ScheduleSettings::default();
                sched.cron = Some(v.to_string());
                sched.validate_cron().map_err(|e| match e {
                    ricochet_core::content::ContentError::InvalidSchedule(ee) => ee.to_string(),
                    _ => "Invalid cron schedule".to_string(),
                })
            })
            .allow_empty(false)
            .with_post_completion_text("Schedule saved!")
            .interact_text()?;
        sched.cron = Some(cron);
    } else {
        sched.cron = Some(opts[opt].to_string());
    }
    Ok(Some(sched))
}

pub fn init_rico_toml(dir: &PathBuf) -> anyhow::Result<ContentItem> {
    let lang = choose_language();
    let content_type = choose_content_type(&lang)?;
    let entrypoint = choose_entrypoint(&content_type, dir)?;
    let schedule = schedule(&content_type)?;
    let static_ = static_settings(dir, &content_type)?;
    let name = choose_item_name();
    let access_type = choose_access_type();

    let packages = Package::from(&lang);

    let language = LanguageConfig {
        name: lang,
        packages,
    };

    let serve = if content_type.is_service() {
        Some(ServeSettings::default())
    } else {
        None
    };

    let res = ContentItem {
        content: Content {
            id: None,
            name,
            slug: None,
            entrypoint,
            access_type,
            content_type,
            summary: None,
            thumbnail: None,
            tags: None,
            include: None,
            exclude: None,
        },
        language,
        env_vars: None,
        schedule,
        serve,
        static_,
    };

    println!("{}", toml::to_string_pretty(&res)?);
    Ok(res)
}
