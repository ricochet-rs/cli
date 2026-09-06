use anyhow::bail;
use colored::Colorize;
use dialoguer::{Confirm, FuzzySelect, Input, Select, theme::ColorfulTheme};
use ricochet_core::{
    content::{AccessType, Content, ContentItem, ContentType},
    language::{Language, LanguageConfig, Package},
    settings::{ScheduleSettings, ServeSettings, StaticSettings},
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::commands::detect::{
    EntrypointCandidate, StaticOutput, find_files_by_extension, find_quarto_projects,
    find_server_yml, find_shiny_dirs,
};

pub fn choose_language() -> Language {
    let languages = [Language::R, Language::Python, Language::Julia];
    let language_names = ["R", "Python", "Julia"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a language")
        .items(language_names)
        .default(0)
        .interact()
        .unwrap_or(0);

    languages[selection]
}

pub fn choose_content_type(language: &Language) -> anyhow::Result<ContentType> {
    let opts = match language {
        Language::R => {
            vec![
                ContentType::R,
                ContentType::RService,
                ContentType::Plumber,
                ContentType::RServer,
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
        Language::Python => vec![
            ContentType::Python,
            ContentType::PythonService,
            ContentType::QuartoPy,
            ContentType::FastApi,
            ContentType::Flask,
            ContentType::Streamlit,
            ContentType::ShinyPy,
            ContentType::Dash,
        ],
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

fn find_candidate_entrypoints(extension: &str, search_dir: &Path) -> anyhow::Result<PathBuf> {
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

fn choose_shiny_entrypoint(dir: &Path) -> anyhow::Result<PathBuf> {
    let all_r_files = find_files_by_extension("R", dir)?;

    // app.R is the conventional entrypoint, so offer it first
    let mut candidates = all_r_files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("app.R"))
        })
        .cloned()
        .collect::<Vec<_>>();

    candidates.extend(find_shiny_dirs(dir));

    // Add remaining .R files that aren't already in the list
    for r_file in &all_r_files {
        if !candidates.contains(r_file) {
            candidates.push(r_file.clone());
        }
    }

    if candidates.is_empty() {
        bail!("No .R files found in {}", dir.display());
    }

    let display_candidates: Vec<String> = candidates
        .iter()
        .map(|p| {
            if p == &PathBuf::from(".") {
                "./ (ui.R + server.R)".to_string()
            } else if p.extension().is_none() {
                format!("{}/ (ui.R + server.R)", p.display())
            } else {
                format!("{}", p.display())
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

fn choose_entrypoint(content_type: &ContentType, dir: &Path) -> anyhow::Result<PathBuf> {
    match content_type {
        ContentType::R
        | ContentType::Plumber
        | ContentType::RService
        | ContentType::ServerlessR
        | ContentType::Ambiorix => find_candidate_entrypoints("R", dir),
        // The standard fixes the name and the location, so there is nothing to choose
        ContentType::RServer => find_server_yml(dir)
            .ok_or_else(|| anyhow::anyhow!("No _server.yml found in {}", dir.display())),
        ContentType::Shiny => choose_shiny_entrypoint(dir),
        ContentType::Rmd | ContentType::RmdShiny => find_candidate_entrypoints("Rmd", dir),
        ContentType::Julia | ContentType::JuliaService => find_candidate_entrypoints("jl", dir),
        ContentType::QuartoR
        | ContentType::QuartoRShiny
        | ContentType::QuartoJl
        | ContentType::QuartoPy => {
            let mut candidates = find_files_by_extension("qmd", dir)?;
            // Include _quarto.yml files for quarto website/book projects
            candidates.extend(find_quarto_projects(dir)?);

            if candidates.is_empty() {
                bail!("No .qmd files or _quarto.yml found in {}", dir.display());
            }

            let display_candidates = candidates.iter().map(|i| i.display()).collect::<Vec<_>>();
            let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select entrypoint file")
                .highlight_matches(true)
                .items(display_candidates)
                .interact()?;

            Ok(candidates[selection].clone())
        }
        ContentType::Python
        | ContentType::PythonService
        | ContentType::FastApi
        | ContentType::Flask
        | ContentType::Streamlit
        | ContentType::ShinyPy
        | ContentType::Dash => find_candidate_entrypoints("py", dir),
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
        .items(opts)
        .default(0)
        .interact()
        .unwrap_or(0);

    opts[selection]
}

fn static_settings(
    path: &Path,
    content_type: &ContentType,
    entrypoint: &Path,
) -> anyhow::Result<Option<StaticSettings>> {
    if !content_type.maybe_static() {
        return Ok(None);
    }

    let theme = ColorfulTheme::default();

    let candidate = EntrypointCandidate {
        path: entrypoint.to_path_buf(),
        content_type: *content_type,
    };
    if let Some(static_output) = StaticOutput::from_quarto(path, &candidate) {
        println!(
            "  {} Detected quarto website project (output: {})",
            "→".bright_cyan(),
            static_output.output_dir.bright_cyan()
        );

        return Ok(Some(StaticSettings {
            index: Some(static_output.index),
            output_dir: Some(static_output.output_dir),
            render_fn: None,
        }));
    }

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
    if !content_type.is_task() {
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

pub fn init_rico_toml(dir: &Path, overwrite: bool, dry_run: bool) -> anyhow::Result<ContentItem> {
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
    let static_ = static_settings(dir, &content_type, &entrypoint)?;
    let name = choose_item_name();
    let access_type = choose_access_type();

    let packages = Package::from(&lang);

    let language = LanguageConfig {
        name: lang,
        packages,
    };

    let serve = if content_type.is_app() {
        Some(ServeSettings::default())
    } else {
        None
    };

    let res = ContentItem {
        content: Content {
            id: Some(ulid::Ulid::generate().to_string()),
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
        repositories: None,
        retention: None,
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

        // Warn if the required package file is missing
        let pkg_path = dir.join(res.language.packages.to_string());
        if !pkg_path.exists() {
            // For uv.lock, check parent dirs (uv workspace support)
            if let Package::UvLock = res.language.packages {
                if let Some(found) = crate::utils::find_in_parent_dirs(dir, "uv.lock") {
                    println!(
                        "  {} Found {} in workspace root (will be included during deploy)",
                        "→".bright_cyan(),
                        found.display().to_string().bright_cyan()
                    );
                } else {
                    eprintln!(
                        "\n{} Required package file `{}` not found. Create it by running `uv init`",
                        "⚠".yellow().bold(),
                        res.language.packages.to_string().bright_cyan(),
                    );
                }
            } else {
                let hint = match res.language.packages {
                    Package::RenvLock => "Create it by running `renv::snapshot()` in R",
                    Package::ManifestToml => "Create it by running `Pkg.instantiate()` in Julia",
                    Package::UvLock => unreachable!(),
                };
                eprintln!(
                    "\n{} Required package file `{}` not found. {}",
                    "⚠".yellow().bold(),
                    res.language.packages.to_string().bright_cyan(),
                    hint
                );
            }
        }
    }

    Ok(res)
}
