use anyhow::bail;
use colored::Colorize;
use dialoguer::{Confirm, FuzzySelect, Input, Select, theme::ColorfulTheme};
use ricochet_core::{
    content::{AccessType, Content, ContentItem, ContentType},
    language::{Language, LanguageConfig, Package},
    settings::{ScheduleSettings, ServeSettings, StaticSettings},
};
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn choose_language() -> Language {
    let languages = [Language::R, Language::Python, Language::Julia];
    let language_names = ["R", "Python", "Julia"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a language")
        .items(language_names)
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

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose content type")
        .highlight_matches(true)
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

// Not case sensitive
fn find_files_by_extension(extension: &str, search_dir: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let extension_lower = extension.to_lowercase();

    let res = WalkDir::new(search_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !entry.file_name().eq("renv"))
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_lowercase() == extension_lower)
                    .unwrap_or(false)
        })
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(search_dir)
                .ok()
                .map(|inner| inner.to_path_buf())
        })
        .collect::<Vec<_>>();

    Ok(res)
}

fn find_candidate_entrypoints(extension: &str, search_dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let candidates = find_files_by_extension(extension, search_dir)?;

    if candidates.is_empty() {
        bail!(
            "No valid entrypoint files found in {}",
            search_dir.display()
        );
    }

    let display_candidates = candidates.iter().map(|i| i.display()).collect::<Vec<_>>();
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select entrypoint file")
        .highlight_matches(true)
        .items(display_candidates)
        .interact()?;

    Ok(candidates[selection].clone())
}

fn choose_shiny_entrypoint(dir: &PathBuf) -> anyhow::Result<PathBuf> {
    let mut candidates = Vec::new();

    // Find all app.R files
    let app_files = find_files_by_extension("R", dir)?.into_iter().filter(|p| {
        p.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("app.R"))
            .unwrap_or(false)
    });
    candidates.extend(app_files);

    // Find directories with both ui.R and server.R
    for entry in WalkDir::new(dir)
        .min_depth(0)
        .max_depth(2)
        .into_iter()
        .filter_entry(|entry| !entry.file_name().eq("renv"))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let ui_path = entry.path().join("ui.R");
        let server_path = entry.path().join("server.R");

        if ui_path.exists() && server_path.exists() {
            // Add the directory path (relative to search_dir)
            if let Ok(relative) = entry.path().strip_prefix(dir) {
                candidates.push(relative.to_path_buf());
            }
        }
    }

    if candidates.is_empty() {
        bail!(
            "No Shiny app found in {}. Looking for app.R or a directory with ui.R and server.R",
            dir.display()
        );
    }

    let display_candidates: Vec<String> = candidates
        .iter()
        .map(|p| {
            if p.to_str() == Some("") || p == &PathBuf::from(".") {
                "./ (ui.R + server.R)".to_string()
            } else if p.file_name().and_then(|n| n.to_str()) == Some("app.R") {
                format!("{}", p.display())
            } else {
                format!("{}/ (ui.R + server.R)", p.display())
            }
        })
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select Shiny app entrypoint")
        .highlight_matches(true)
        .items(&display_candidates)
        .interact()?;

    Ok(candidates[selection].clone())
}

fn choose_entrypoint(content_type: &ContentType, dir: &PathBuf) -> anyhow::Result<PathBuf> {
    match content_type {
        ContentType::R
        | ContentType::Plumber
        | ContentType::RService
        | ContentType::ServerlessR
        | ContentType::Ambiorix => find_candidate_entrypoints("R", dir),
        ContentType::Shiny => choose_shiny_entrypoint(dir),
        ContentType::Rmd | ContentType::RmdShiny => find_candidate_entrypoints("Rmd", dir),
        ContentType::Julia | ContentType::JuliaService => find_candidate_entrypoints("jl", dir),
        ContentType::QuartoR | ContentType::QuartoRShiny | ContentType::QuartoJl => {
            find_candidate_entrypoints("qmd", dir)
        }
        ContentType::ServerlessJl | ContentType::Python | ContentType::PythonService => {
            bail!("Requested content type not yet implemented")
        }
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
                let sched = ScheduleSettings {
                    cron: Some(v.to_string()),
                    ..Default::default()
                };

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

pub fn init_rico_toml(
    dir: &PathBuf,
    overwrite: bool,
    dry_run: bool,
) -> anyhow::Result<ContentItem> {
    // Check for non-interactive mode (tests, CI, etc.)
    if crate::utils::is_non_interactive() {
        bail!(
            "Cannot run init in non-interactive mode. Please create _ricochet.toml manually or run `ricochet init` interactively."
        );
    }

    // Check if _ricochet.toml already exists
    let toml_path = dir.join("_ricochet.toml");

    if !dry_run && toml_path.exists() && !overwrite {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "_ricochet.toml already exists at {}. Overwrite?",
                toml_path.display()
            ))
            .default(false)
            .interact()?;

        if !confirmed {
            bail!("Cancelled: _ricochet.toml already exists");
        }
    }

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
            exec_env: None,
        },
        language,
        env_vars: None,
        schedule,
        serve,
        static_,
        resources: None,
    };

    let toml_content = toml::to_string_pretty(&res)?;

    if dry_run {
        // Only print to terminal, don't save
        println!("{}", toml_content);
    } else {
        std::fs::write(&toml_path, &toml_content)?;
        println!(
            "{} Created _ricochet.toml",
            unicode_icons::icons::symbols::check_mark().0.green()
        );
    }

    Ok(res)
}
