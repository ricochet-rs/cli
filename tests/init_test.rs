//! `ricochet init` answered entirely by flags, as an editor or script runs it.

use std::path::Path;
use std::process::Output;
use tempfile::TempDir;

fn init(dir: &Path, args: &[&str]) -> Output {
    let home = TempDir::new().expect("creating a temporary home");
    std::process::Command::new(env!("CARGO_BIN_EXE_ricochet"))
        .arg("init")
        .arg(dir)
        .args(args)
        .env("HOME", home.path())
        .env("RICOCHET_NO_UPDATE_CHECK", "1")
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("running the ricochet binary")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Every content type with an entrypoint it accepts, so `validate_config` passes for each.
const CASES: &[(&str, &str)] = &[
    ("r", "job.R"),
    ("r-service", "service.R"),
    ("plumber", "api.R"),
    ("r-server", "_server.yml"),
    ("ambiorix", "app.R"),
    ("shiny", "app.R"),
    ("rmd", "report.Rmd"),
    ("rmd-shiny", "report.Rmd"),
    ("serverless-r", "handler.R"),
    ("quarto-r", "report.qmd"),
    ("quarto-r-shiny", "report.qmd"),
    ("julia", "job.jl"),
    ("julia-service", "service.jl"),
    ("quarto-jl", "report.qmd"),
    ("python", "job.py"),
    ("python-service", "service.py"),
    ("quarto-py", "report.qmd"),
    ("jupyter", "analysis.ipynb"),
    ("fast-api", "main.py"),
    ("flask", "app.py"),
    ("streamlit", "app.py"),
    ("shiny-py", "app.py"),
    ("dash", "app.py"),
];

#[test]
fn every_content_type_initializes_without_a_terminal() {
    for (content_type, entrypoint) in CASES {
        let dir = TempDir::new().expect("creating project");
        std::fs::write(dir.path().join(entrypoint), "engine: plumber2\n")
            .expect("writing entrypoint");

        let output = init(
            dir.path(),
            &[
                "--content-type",
                content_type,
                "--entrypoint",
                entrypoint,
                "--name",
                "Item",
                "--access-type",
                "private",
            ],
        );
        assert!(
            output.status.success(),
            "{content_type}: {}",
            stderr(&output)
        );

        let written = std::fs::read_to_string(dir.path().join("_ricochet.toml"))
            .expect("reading _ricochet.toml");
        let item: toml::Value = toml::from_str(&written).expect("parsing _ricochet.toml");
        assert_eq!(
            item["content"]["content_type"].as_str(),
            Some(*content_type)
        );
        assert_eq!(item["content"]["entrypoint"].as_str(), Some(*entrypoint));
        assert!(
            item.get("schedule").is_none(),
            "{content_type} was scheduled"
        );
    }
}

#[test]
fn missing_answers_are_named_rather_than_prompted_for() {
    let dir = TempDir::new().expect("creating project");
    let output = init(dir.path(), &["--content-type", "shiny"]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("Pass --entrypoint, --name, --access-type"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.path().join("_ricochet.toml").exists());
}

#[test]
fn an_existing_config_is_kept_without_overwrite() {
    let dir = TempDir::new().expect("creating project");
    std::fs::write(dir.path().join("app.R"), "").expect("writing app.R");
    std::fs::write(dir.path().join("_ricochet.toml"), "kept").expect("writing _ricochet.toml");
    let answers = [
        "--content-type",
        "shiny",
        "--entrypoint",
        "app.R",
        "--name",
        "Item",
        "--access-type",
        "private",
    ];

    let refused = init(dir.path(), &answers);
    assert!(!refused.status.success());
    assert!(
        stderr(&refused).contains("Pass --overwrite"),
        "{}",
        stderr(&refused)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("_ricochet.toml")).expect("reading"),
        "kept"
    );

    let replaced = init(dir.path(), &[&answers[..], &["--overwrite"]].concat());
    assert!(replaced.status.success(), "{}", stderr(&replaced));
}

#[test]
fn an_entrypoint_that_does_not_fit_the_content_type_is_rejected() {
    let dir = TempDir::new().expect("creating project");
    std::fs::write(dir.path().join("app.py"), "").expect("writing app.py");

    let output = init(
        dir.path(),
        &[
            "--content-type",
            "shiny",
            "--entrypoint",
            "app.py",
            "--name",
            "Item",
            "--access-type",
            "private",
        ],
    );
    assert!(!output.status.success());
    assert!(!dir.path().join("_ricochet.toml").exists());
}
