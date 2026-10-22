mod prefs;

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail, ensure};
use tempfile::Builder as TempFileBuilder;

use prefs::{Preference, parse, render};

const DEFAULT_INPUT_DIR: &str = "prefs";
const DEFAULT_OUTPUT_FILE: &str = "dist/prefs/aion.js";
const DEFAULT_ENGINE_DIR: &str = "engine";
const INPUT_EXTENSION: &str = "toml";
const AION_INCLUDE: &str = "#include aion.js";

#[derive(Debug)]
struct Options {
    input: PathBuf,
    output: Option<PathBuf>,
    engine: Option<PathBuf>,
    apply: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            input: PathBuf::from(DEFAULT_INPUT_DIR),
            output: None,
            engine: None,
            apply: false,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aion-prefs: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut options = Options::default();
    let mut args = env::args_os().skip(1);
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "Usage: aion-prefs [--input DIRECTORY] [--output FILE]\n\
                     Or:    aion-prefs --apply [--input DIRECTORY] [--engine DIRECTORY]\n\
                     Defaults: --input {DEFAULT_INPUT_DIR} --output {DEFAULT_OUTPUT_FILE}\n\n\
                     Reads TOML files recursively and generates Firefox defaults.\n\
                     --apply writes preferences into {DEFAULT_ENGINE_DIR}/ and installs the include.\n\
                     --engine overrides the checkout location when applying.\n\
                     Run from the Aion root, or provide explicit paths."
                );
                return Ok(());
            }
            Some("--input") => {
                options.input = path_argument(&mut args, "--input")?;
            }
            Some("--output") => {
                options.output = Some(path_argument(&mut args, "--output")?);
            }
            Some("--engine") => {
                options.engine = Some(path_argument(&mut args, "--engine")?);
            }
            Some("--apply") => options.apply = true,
            _ => bail!("unknown argument {argument:?}; use --help"),
        }
    }

    if options.apply && options.output.is_some() {
        bail!("--apply and --output cannot be combined; use --engine to select the checkout");
    }
    if !options.apply && options.engine.is_some() {
        bail!("--engine requires --apply");
    }
    let preferences = load_preferences(&options.input)?;
    let content = render(&preferences);
    if options.apply {
        let engine = options
            .engine
            .as_deref()
            .unwrap_or_else(|| Path::new(DEFAULT_ENGINE_DIR));
        let output = apply_to_engine(engine, &content)?;
        println!(
            "Applied {} preferences to {}",
            preferences.len(),
            output.display()
        );
        return Ok(());
    }
    let output = options
        .output
        .as_deref()
        .unwrap_or_else(|| Path::new(DEFAULT_OUTPUT_FILE));
    validate_output_path(output)?;
    write_output(output, &content)?;
    println!(
        "Generated {} preferences in {}",
        preferences.len(),
        output.display()
    );
    Ok(())
}

fn apply_to_engine(engine: &Path, content: &str) -> Result<PathBuf> {
    let firefox = engine.join("browser/app/profile/firefox.js");
    // Reject missing checkouts before creating any files
    let original = fs::read_to_string(&firefox).with_context(|| {
        format!(
            "cannot read {}; --apply requires a Firefox checkout",
            firefox.display()
        )
    })?;
    let output = engine.join("browser/app/profile/aion.js");
    validate_output_path(&output)?;
    let updated = with_aion_include(&original);
    // Write the target before adding its include
    write_output(&output, content)?;
    if updated != original {
        write_output(&firefox, &updated)?;
    }
    Ok(output)
}

fn with_aion_include(source: &str) -> String {
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut updated = String::new();
    // Move existing includes outside platform conditions
    for line in source.split_inclusive('\n') {
        if line.trim() != AION_INCLUDE {
            updated.push_str(line);
        }
    }
    if !updated.ends_with('\n') {
        updated.push_str(newline);
    }
    if !updated.ends_with(&format!("{newline}{newline}")) {
        updated.push_str(newline);
    }
    updated.push_str(AION_INCLUDE);
    updated.push_str(newline);
    updated
}

fn path_argument(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<PathBuf> {
    args.next()
        .filter(|value| !value.is_empty() && !value.as_encoded_bytes().starts_with(b"-"))
        .map(PathBuf::from)
        .with_context(|| {
            format!("{option} requires a path; prefix paths starting with '-' with './'")
        })
}

fn validate_output_path(output: &Path) -> Result<()> {
    let resolved = match output.canonicalize() {
        Ok(path) => Some(path),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("cannot resolve {}", output.display()));
        }
    };
    // Reject source overwrites and output that would become input on the next run
    if output
        .extension()
        .is_some_and(|extension| extension == INPUT_EXTENSION)
        || resolved.as_ref().is_some_and(|path| {
            path.extension()
                .is_some_and(|extension| extension == INPUT_EXTENSION)
        })
    {
        bail!(
            "{}: generated output cannot use a TOML configuration file",
            output.display()
        );
    }
    Ok(())
}

fn write_output(output: &Path, content: &str) -> Result<()> {
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("cannot create {}", parent.display()))?;
    // Stage on the same filesystem for atomic replacement; honor the process umask
    let mut staged = TempFileBuilder::new()
        .prefix(".aion-prefs-")
        .make_in(parent, |path| {
            OpenOptions::new().write(true).create_new(true).open(path)
        })
        .with_context(|| format!("cannot create temporary output in {}", parent.display()))?;
    staged
        .write_all(content.as_bytes())
        .with_context(|| format!("cannot write temporary output for {}", output.display()))?;
    if let Ok(metadata) = fs::metadata(output) {
        staged
            .as_file()
            .set_permissions(metadata.permissions())
            .with_context(|| format!("cannot preserve permissions for {}", output.display()))?;
    }
    staged
        .persist(output)
        .with_context(|| format!("cannot replace {}", output.display()))?;
    Ok(())
}

fn load_preferences(input: &Path) -> Result<Vec<Preference>> {
    let mut files = Vec::new();
    collect_files(input, &mut files)?;
    files.sort();
    ensure!(
        !files.is_empty(),
        "no TOML files found in {}",
        input.display()
    );

    let mut preferences: BTreeMap<String, (Preference, PathBuf)> = BTreeMap::new();
    for file in files {
        let content =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        for preference in
            parse(&content).with_context(|| format!("cannot parse {}", file.display()))?
        {
            if let Some((_, previous)) = preferences.get(&preference.name) {
                bail!(
                    "{}: duplicate preference {:?}; previously defined in {}",
                    file.display(),
                    preference.name,
                    previous.display()
                );
            }
            preferences.insert(preference.name.clone(), (preference, file.clone()));
        }
    }
    ensure!(!preferences.is_empty(), "input contains no preferences");
    Ok(preferences.into_values().map(|(pref, _)| pref).collect())
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("cannot read directory {}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("cannot read entry in {}", directory.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .with_context(|| format!("cannot inspect {}", path.display()))?;
        if kind.is_symlink() {
            bail!(
                "{}: symlinks are not supported in the input directory",
                path.display()
            );
        }
        if kind.is_dir() {
            collect_files(&path, files)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == INPUT_EXTENSION)
        {
            files.push(path);
        }
    }
    Ok(())
}
