//! Non-interactive inspection of a project directory.
//!
//! [`Detection::scan`] reports every plausible entrypoint together with the
//! content type it implies. It never picks one and never writes a file, so the
//! same code can back `ricochet detect`, `ricochet init` and any program that
//! drives the CLI over `-F json`.

use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{Cell, Table, presets::UTF8_FULL};
use ricochet_core::{
    content::ContentType,
    kinds::QuartoYml,
    language::{Language, Package},
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::OutputFormat;

/// How much of a source file is read when looking for framework markers.
const HEAD_BYTES: usize = 16 * 1024;

/// A file or directory that could serve as the entrypoint of a content item.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct EntrypointCandidate {
    /// Path relative to the inspected directory.
    pub path: PathBuf,
    pub content_type: ContentType,
}

/// The package manifest the detected language requires.
#[derive(Serialize, Clone, Debug)]
pub struct PackageFile {
    pub kind: Package,
    pub path: PathBuf,
    pub found: bool,
}

/// The rendered site a Quarto project declares in its `_quarto.yml`.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct StaticOutput {
    pub output_dir: String,
    pub index: String,
}

/// Everything `ricochet init` would otherwise ask a person about.
#[derive(Serialize, Clone, Debug)]
pub struct Detection {
    /// Language of the highest-ranked entrypoint, absent when nothing was found.
    pub language: Option<Language>,
    /// Distinct content types across [`Detection::entrypoints`].
    pub content_types: Vec<ContentType>,
    pub entrypoints: Vec<EntrypointCandidate>,
    pub package_file: Option<PackageFile>,
    #[serde(rename = "static")]
    pub static_: Option<StaticOutput>,
}

impl Detection {
    pub fn scan(dir: &Path) -> Result<Self> {
        let dir = dir
            .canonicalize()
            .with_context(|| format!("Cannot inspect {}", dir.display()))?;

        let mut entrypoints = Vec::new();

        for path in find_shiny_dirs(&dir) {
            entrypoints.push(EntrypointCandidate {
                path,
                content_type: ContentType::Shiny,
            });
        }

        for path in find_files_by_extension("R", &dir)? {
            let content_type = if file_name_eq(&path, "app.R") {
                ContentType::Shiny
            } else {
                classify_r_script(&dir.join(&path))
            };
            entrypoints.push(EntrypointCandidate { path, content_type });
        }

        for path in find_files_by_extension("Rmd", &dir)? {
            let content_type = classify_rmd(&dir.join(&path));
            entrypoints.push(EntrypointCandidate { path, content_type });
        }

        for path in find_files_by_extension("qmd", &dir)? {
            let content_type = classify_qmd(&dir.join(&path));
            entrypoints.push(EntrypointCandidate { path, content_type });
        }

        for path in find_quarto_projects(&dir)? {
            let content_type = classify_quarto_project(&dir, &path);
            entrypoints.push(EntrypointCandidate { path, content_type });
        }

        for path in find_files_by_extension("py", &dir)? {
            let content_type = classify_python_script(&dir.join(&path));
            entrypoints.push(EntrypointCandidate { path, content_type });
        }

        for path in find_files_by_extension("jl", &dir)? {
            entrypoints.push(EntrypointCandidate {
                path,
                content_type: ContentType::Julia,
            });
        }

        entrypoints.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.path.cmp(&b.path)));

        let mut content_types: Vec<ContentType> = Vec::new();
        for candidate in &entrypoints {
            if !content_types.contains(&candidate.content_type) {
                content_types.push(candidate.content_type);
            }
        }

        let language = entrypoints
            .first()
            .map(|candidate| Language::from(&candidate.content_type));
        let package_file = language
            .as_ref()
            .map(|language| PackageFile::locate(&dir, Package::from(language)));
        let static_ = entrypoints
            .iter()
            .find_map(|candidate| StaticOutput::from_quarto(&dir, candidate));

        Ok(Self {
            language,
            content_types,
            entrypoints,
            package_file,
            static_,
        })
    }
}

impl PackageFile {
    fn locate(dir: &Path, kind: Package) -> Self {
        let name = kind.to_string();

        if dir.join(&name).exists() {
            return Self {
                kind,
                path: PathBuf::from(name),
                found: true,
            };
        }

        // uv workspaces keep a single lockfile at the workspace root
        if let Package::UvLock = kind
            && let Some(found) = crate::utils::find_in_parent_dirs(dir, &name)
        {
            return Self {
                kind,
                path: found,
                found: true,
            };
        }

        Self {
            kind,
            path: PathBuf::from(name),
            found: false,
        }
    }
}

impl StaticOutput {
    /// Read `project.output-dir` from the `_quarto.yml` belonging to a candidate.
    pub fn from_quarto(dir: &Path, candidate: &EntrypointCandidate) -> Option<Self> {
        if !matches!(
            candidate.content_type,
            ContentType::QuartoR | ContentType::QuartoJl | ContentType::QuartoPy
        ) {
            return None;
        }

        let entrypoint_dir = candidate
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();

        let quarto = QuartoYml::from_file(dir.join(&entrypoint_dir).join("_quarto.yml")).ok()?;
        let output_dir = quarto.project.as_ref()?.get_output_dir()?;

        let output_dir = if entrypoint_dir.as_os_str().is_empty() {
            output_dir
        } else {
            entrypoint_dir.join(output_dir)
        };

        Some(Self {
            output_dir: output_dir.display().to_string(),
            index: "index.html".to_string(),
        })
    }
}

/// Files with the given extension, case-insensitively, relative to `search_dir`.
pub fn find_files_by_extension(extension: &str, search_dir: &Path) -> Result<Vec<PathBuf>> {
    let extension_lower = extension.to_lowercase();

    let res = WalkDir::new(search_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name();
            !name.eq("renv") && !name.eq(".venv") && !name.eq("venv") && !name.eq("env")
        })
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

/// Directories holding both `ui.R` and `server.R`, which Shiny accepts as an entrypoint.
pub fn find_shiny_dirs(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .min_depth(0)
        .max_depth(2)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| !entry.file_name().eq("renv"))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
        .filter(|entry| {
            entry.path().join("ui.R").exists() && entry.path().join("server.R").exists()
        })
        .filter_map(|entry| entry.path().strip_prefix(dir).ok().map(Path::to_path_buf))
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                relative
            }
        })
        .collect()
}

/// `_quarto.yml` files, which are the entrypoint of a Quarto website or book.
pub fn find_quarto_projects(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(find_files_by_extension("yml", dir)?
        .into_iter()
        .filter(|path| file_name_eq(path, "_quarto.yml"))
        .collect())
}

fn file_name_eq(path: &Path, name: &str) -> bool {
    path.file_name()
        .and_then(|inner| inner.to_str())
        .is_some_and(|inner| inner.eq_ignore_ascii_case(name))
}

/// Conventional entrypoints sort ahead of everything else.
fn rank(candidate: &EntrypointCandidate) -> u8 {
    if candidate.content_type == ContentType::Shiny && candidate.path.extension().is_none() {
        return 0;
    }

    let name = candidate
        .path
        .file_name()
        .and_then(|inner| inner.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match name.as_str() {
        "app.r" | "app.py" | "plumber.r" | "_quarto.yml" | "index.qmd" | "index.rmd"
        | "main.py" | "main.jl" => 0,
        _ => 1,
    }
}

/// The first [`HEAD_BYTES`] of a file, lossily decoded, empty when unreadable.
fn read_head(path: &Path) -> String {
    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let mut buf = vec![0u8; HEAD_BYTES];
    let Ok(read) = file.read(&mut buf) else {
        return String::new();
    };
    buf.truncate(read);
    String::from_utf8_lossy(&buf).into_owned()
}

fn attaches(head: &str, package: &str) -> bool {
    head.contains(&format!("library({package})")) || head.contains(&format!("{package}::"))
}

/// Match the whole module name, so `flask_login` does not read as `flask`.
fn imports(head: &str, module: &str) -> bool {
    [format!("import {module}"), format!("from {module}")]
        .iter()
        .any(|statement| {
            head.match_indices(statement.as_str())
                .any(|(start, matched)| {
                    head[start + matched.len()..]
                        .chars()
                        .next()
                        .is_none_or(|next| !next.is_alphanumeric() && next != '_')
                })
        })
}

fn classify_r_script(path: &Path) -> ContentType {
    let head = read_head(path);

    if attaches(&head, "plumber") || head.contains("#* @") {
        ContentType::Plumber
    } else if attaches(&head, "ambiorix") {
        ContentType::Ambiorix
    } else if attaches(&head, "shiny") || head.contains("shinyApp(") {
        ContentType::Shiny
    } else {
        ContentType::R
    }
}

fn classify_python_script(path: &Path) -> ContentType {
    let head = read_head(path);

    if imports(&head, "streamlit") {
        ContentType::Streamlit
    } else if imports(&head, "fastapi") {
        ContentType::FastApi
    } else if imports(&head, "flask") {
        ContentType::Flask
    } else if imports(&head, "shiny") {
        ContentType::ShinyPy
    } else if imports(&head, "dash") {
        ContentType::Dash
    } else {
        ContentType::Python
    }
}

fn classify_rmd(path: &Path) -> ContentType {
    if read_head(path).contains("runtime: shiny") {
        ContentType::RmdShiny
    } else {
        ContentType::Rmd
    }
}

fn classify_qmd(path: &Path) -> ContentType {
    let head = read_head(path);

    if head.contains("server: shiny") {
        return ContentType::QuartoRShiny;
    }

    match quarto_language(&head) {
        Language::Julia => ContentType::QuartoJl,
        Language::Python => ContentType::QuartoPy,
        Language::R => ContentType::QuartoR,
    }
}

/// A project takes the content type of its own `engine:`, else of its first document.
fn classify_quarto_project(dir: &Path, yml: &Path) -> ContentType {
    let project_dir = yml.parent().map(Path::to_path_buf).unwrap_or_default();
    let head = read_head(&dir.join(yml));

    if head.contains("engine:") {
        return match quarto_language(&head) {
            Language::Julia => ContentType::QuartoJl,
            Language::Python => ContentType::QuartoPy,
            Language::R => ContentType::QuartoR,
        };
    }

    find_files_by_extension("qmd", &dir.join(&project_dir))
        .ok()
        .and_then(|docs| docs.first().cloned())
        .map(|doc| classify_qmd(&dir.join(&project_dir).join(doc)))
        .unwrap_or(ContentType::QuartoR)
}

/// Quarto names its language in `engine:`, or implies it with the first code chunk.
fn quarto_language(head: &str) -> Language {
    if head.contains("engine: julia") || head.contains("```{julia}") {
        Language::Julia
    } else if head.contains("engine: jupyter") || head.contains("```{python}") {
        Language::Python
    } else {
        Language::R
    }
}

pub fn detect(dir: &Path, format: OutputFormat) -> Result<()> {
    let detection = Detection::scan(dir)?;

    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&detection)?),
        OutputFormat::Yaml => println!("{}", serde_yaml::to_string(&detection)?),
        OutputFormat::Table => print_detection(&detection),
    }

    Ok(())
}

fn print_detection(detection: &Detection) {
    let Some(language) = detection.language else {
        println!("No deployable content found.");
        return;
    };

    println!("{} {}", "Language:".bold(), language);

    let content_types = detection
        .content_types
        .iter()
        .map(ContentType::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    println!("{} {}", "Content types:".bold(), content_types);

    if let Some(package_file) = &detection.package_file {
        let state = if package_file.found {
            package_file.path.display().to_string().green()
        } else {
            format!("{} (missing)", package_file.kind).yellow()
        };
        println!("{} {state}", "Packages:".bold());
    }

    if let Some(static_) = &detection.static_ {
        println!(
            "{} {}/{}",
            "Static output:".bold(),
            static_.output_dir,
            static_.index
        );
    }

    let mut table = Table::new();
    table.load_style(UTF8_FULL);
    table.set_header(vec![Cell::new("Entrypoint"), Cell::new("Content type")]);
    for candidate in &detection.entrypoints {
        table.add_row(vec![
            Cell::new(candidate.path.display().to_string()),
            Cell::new(candidate.content_type.to_string()),
        ]);
    }
    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, path: &str, contents: &str) {
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(full, contents).expect("write file");
    }

    fn scan(dir: &TempDir) -> Detection {
        Detection::scan(dir.path()).expect("scan")
    }

    #[test]
    fn empty_directory_detects_nothing() {
        let dir = TempDir::new().expect("tempdir");
        let detection = scan(&dir);

        assert!(detection.language.is_none());
        assert!(detection.content_types.is_empty());
        assert!(detection.entrypoints.is_empty());
        assert!(detection.package_file.is_none());
        assert!(detection.static_.is_none());
    }

    #[test]
    fn app_r_detects_a_shiny_app() {
        let dir = TempDir::new().expect("tempdir");
        write(
            dir.path(),
            "app.R",
            "library(shiny)\nshinyApp(ui, server)\n",
        );
        write(dir.path(), "renv.lock", "{}");

        let detection = scan(&dir);

        assert_eq!(detection.language, Some(Language::R));
        assert_eq!(detection.content_types, vec![ContentType::Shiny]);
        assert_eq!(
            detection.entrypoints,
            vec![EntrypointCandidate {
                path: PathBuf::from("app.R"),
                content_type: ContentType::Shiny,
            }]
        );
        let package_file = detection.package_file.expect("package file");
        assert!(package_file.found);
        assert_eq!(package_file.path, PathBuf::from("renv.lock"));
    }

    #[test]
    fn a_ui_and_server_directory_is_a_shiny_entrypoint() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "myapp/ui.R", "fluidPage()\n");
        write(dir.path(), "myapp/server.R", "function(input, output) {}\n");

        let detection = scan(&dir);

        assert_eq!(
            detection.entrypoints.first().expect("candidate"),
            &EntrypointCandidate {
                path: PathBuf::from("myapp"),
                content_type: ContentType::Shiny,
            }
        );
    }

    #[test]
    fn plumber_annotations_beat_a_plain_r_script() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "plumber.R", "#* @get /echo\nfunction() {}\n");
        write(dir.path(), "helpers.R", "add <- function(a, b) a + b\n");

        let detection = scan(&dir);

        assert_eq!(
            detection.entrypoints,
            vec![
                EntrypointCandidate {
                    path: PathBuf::from("plumber.R"),
                    content_type: ContentType::Plumber,
                },
                EntrypointCandidate {
                    path: PathBuf::from("helpers.R"),
                    content_type: ContentType::R,
                },
            ]
        );
    }

    #[test]
    fn python_frameworks_are_read_from_imports() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "app.py", "import streamlit as st\n");
        write(dir.path(), "api.py", "from fastapi import FastAPI\n");
        write(dir.path(), "job.py", "print('hello')\n");

        let detection = scan(&dir);

        assert_eq!(detection.language, Some(Language::Python));
        assert_eq!(
            detection.entrypoints,
            vec![
                EntrypointCandidate {
                    path: PathBuf::from("app.py"),
                    content_type: ContentType::Streamlit,
                },
                EntrypointCandidate {
                    path: PathBuf::from("api.py"),
                    content_type: ContentType::FastApi,
                },
                EntrypointCandidate {
                    path: PathBuf::from("job.py"),
                    content_type: ContentType::Python,
                },
            ]
        );
        let package_file = detection.package_file.expect("package file");
        assert!(!package_file.found);
        assert_eq!(package_file.path, PathBuf::from("uv.lock"));
    }

    #[test]
    fn a_module_prefix_is_not_the_framework() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "report.py", "import dashboard_utils\n");
        write(
            dir.path(),
            "helper.py",
            "from flask_admin_stub import thing\n",
        );
        write(dir.path(), "tools.py", "import fastapi_stub\n");

        let detection = scan(&dir);

        assert_eq!(detection.content_types, vec![ContentType::Python]);
    }

    #[test]
    fn a_submodule_still_names_its_framework() {
        let dir = TempDir::new().expect("tempdir");
        write(
            dir.path(),
            "api.py",
            "from fastapi.responses import JSONResponse\n",
        );

        let detection = scan(&dir);

        assert_eq!(detection.content_types, vec![ContentType::FastApi]);
    }

    #[test]
    fn a_quarto_project_reports_its_output_dir() {
        let dir = TempDir::new().expect("tempdir");
        write(
            dir.path(),
            "_quarto.yml",
            "project:\n  type: website\n  output-dir: docs\n",
        );
        write(
            dir.path(),
            "index.qmd",
            "---\ntitle: Home\n---\n\n```{r}\n1\n```\n",
        );

        let detection = scan(&dir);

        assert_eq!(detection.language, Some(Language::R));
        assert_eq!(detection.content_types, vec![ContentType::QuartoR]);
        let static_ = detection.static_.expect("static output");
        assert_eq!(static_.output_dir, "docs");
        assert_eq!(static_.index, "index.html");
    }

    #[test]
    fn a_jupyter_engine_makes_a_quarto_document_python() {
        let dir = TempDir::new().expect("tempdir");
        write(
            dir.path(),
            "report.qmd",
            "---\ntitle: Report\nengine: jupyter\n---\n",
        );

        let detection = scan(&dir);

        assert_eq!(detection.language, Some(Language::Python));
        assert_eq!(detection.content_types, vec![ContentType::QuartoPy]);
    }

    #[test]
    fn a_shiny_runtime_makes_an_rmd_interactive() {
        let dir = TempDir::new().expect("tempdir");
        write(
            dir.path(),
            "dashboard.Rmd",
            "---\ntitle: Dashboard\nruntime: shiny\n---\n",
        );

        let detection = scan(&dir);

        assert_eq!(detection.content_types, vec![ContentType::RmdShiny]);
        assert!(detection.static_.is_none());
    }

    #[test]
    fn every_candidate_is_reported_for_a_mixed_directory() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "app.R", "library(shiny)\n");
        write(dir.path(), "report.qmd", "---\ntitle: Report\n---\n");

        let detection = scan(&dir);

        assert_eq!(
            detection.entrypoints,
            vec![
                EntrypointCandidate {
                    path: PathBuf::from("app.R"),
                    content_type: ContentType::Shiny,
                },
                EntrypointCandidate {
                    path: PathBuf::from("report.qmd"),
                    content_type: ContentType::QuartoR,
                },
            ]
        );
        assert_eq!(
            detection.content_types,
            vec![ContentType::Shiny, ContentType::QuartoR]
        );
    }

    #[test]
    fn virtual_environments_are_skipped() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), ".venv/lib/site.py", "import flask\n");
        write(dir.path(), "renv/activate.R", "library(shiny)\n");
        write(dir.path(), "main.py", "print('hello')\n");

        let detection = scan(&dir);

        assert_eq!(
            detection.entrypoints,
            vec![EntrypointCandidate {
                path: PathBuf::from("main.py"),
                content_type: ContentType::Python,
            }]
        );
    }

    #[test]
    fn a_uv_lock_in_the_workspace_root_counts_as_found() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "uv.lock", "");
        write(dir.path(), "member/main.py", "print('hello')\n");

        let detection = Detection::scan(&dir.path().join("member")).expect("scan");

        let package_file = detection.package_file.expect("package file");
        assert!(package_file.found);
        assert!(package_file.path.ends_with("uv.lock"));
    }

    #[test]
    fn json_uses_the_documented_field_names() {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "report.qmd", "---\ntitle: Report\n---\n");
        write(dir.path(), "renv.lock", "{}");

        let json = serde_json::to_value(scan(&dir)).expect("serialize");

        assert_eq!(json["language"], "r");
        assert_eq!(json["content_types"][0], "quarto-r");
        assert_eq!(json["entrypoints"][0]["path"], "report.qmd");
        assert_eq!(json["entrypoints"][0]["content_type"], "quarto-r");
        assert_eq!(json["package_file"]["kind"], "renv.lock");
        assert_eq!(json["package_file"]["found"], true);
    }
}
