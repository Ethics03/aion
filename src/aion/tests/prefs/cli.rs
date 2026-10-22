use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::process::{Command, Output};

use tempfile::{TempDir, tempdir};

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        fs::create_dir(directory.path().join("prefs")).unwrap();
        Self(directory)
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.0.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aion-prefs"))
            .current_dir(self.0.path())
            .args(arguments)
            .output()
            .unwrap()
    }
}

#[test]
fn generates_sorted_preferences_recursively_without_firefox_checkout() {
    let fixture = Fixture::new();
    fixture.write(
        "prefs/z.toml",
        "[[pref]]\nname = 'z.locked'\nvalue = false\nlocked = true\n",
    );
    fixture.write(
        "prefs/nested/a.toml",
        "[[pref]]\nname = 'a.string'\nvalue = '007'\n",
    );
    let result = fixture.run(&[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let generated = fs::read(fixture.0.path().join("dist/prefs/aion.js")).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&generated)
            .lines()
            .filter(|line| line.starts_with("pref("))
            .collect::<Vec<_>>(),
        [
            "pref(\"a.string\", \"007\");",
            "pref(\"z.locked\", false, locked);"
        ]
    );
    assert!(fixture.run(&[]).status.success());
    assert_eq!(
        fs::read(fixture.0.path().join("dist/prefs/aion.js")).unwrap(),
        generated
    );
}

#[test]
fn invalid_inputs_preserve_existing_output_and_report_source() {
    for (invalid, cause) in [
        (
            "[[pref]]\nname = 'a.valid'\nvalue = true\n",
            "duplicate preference",
        ),
        (
            "[[pref]]\nname = 'z.invalid'\nvalue = 2147483648\n",
            "signed 32 bits",
        ),
        (
            "[[pref]]\nname = 'z.invalid'\nvalue = false\nlock = true\n",
            "unknown field",
        ),
        (
            "[[pref]]\nname = 'z.invalid'\nvalue = '\n",
            "TOML parse error",
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(
            "prefs/a.toml",
            "[[pref]]\nname = 'a.valid'\nvalue = false\n",
        );
        fixture.write("prefs/z.toml", invalid);
        fixture.write("dist/prefs/aion.js", "existing output");
        let result = fixture.run(&[]);
        assert!(!result.status.success());
        let diagnostic = String::from_utf8_lossy(&result.stderr);
        assert!(diagnostic.contains("z.toml"), "{diagnostic}");
        assert!(
            diagnostic.contains(cause),
            "missing underlying cause {cause}: {diagnostic}"
        );
        assert_eq!(
            fs::read_to_string(fixture.0.path().join("dist/prefs/aion.js")).unwrap(),
            "existing output"
        );
    }
}

#[test]
fn supports_explicit_paths_and_rejects_empty_input() {
    let fixture = Fixture::new();
    assert!(!fixture.run(&[]).status.success());
    assert!(!fixture.0.path().join("dist").exists());
    fixture.write(
        "settings/prefs.toml",
        "[[pref]]\nname = 'example'\nvalue = 42\n",
    );
    let result = fixture.run(&["--input", "settings", "--output", "custom.js"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        fs::read_to_string(fixture.0.path().join("custom.js"))
            .unwrap()
            .contains("pref(\"example\", 42);")
    );
}

#[test]
fn missing_path_arguments_do_not_consume_options_or_write_output() {
    let fixture = Fixture::new();
    fixture.write(
        "prefs/privacy.toml",
        "[[pref]]\nname = 'example'\nvalue = false\n",
    );
    for args in [
        &["--input"][..],
        &["--output"][..],
        &["--output", "--input"][..],
        &["--input", "--output"][..],
        &["--output", ""][..],
    ] {
        assert!(!fixture.run(args).status.success());
    }
    assert!(!fixture.0.path().join("--input").exists());
    assert!(!fixture.0.path().join("dist").exists());
}

#[test]
fn output_cannot_overwrite_source_or_become_an_input_on_the_next_run() {
    let fixture = Fixture::new();
    let original = "[[pref]]\nname = 'example'\nvalue = false\n";
    fixture.write("prefs/privacy.toml", original);
    for output in ["prefs/privacy.toml", "prefs/generated.toml"] {
        assert!(!fixture.run(&["--output", output]).status.success());
        assert_eq!(
            fs::read_to_string(fixture.0.path().join("prefs/privacy.toml")).unwrap(),
            original
        );
    }
    assert!(!fixture.0.path().join("prefs/generated.toml").exists());
    #[cfg(unix)]
    {
        symlink("prefs/privacy.toml", fixture.0.path().join("alias.js")).unwrap();
        assert!(!fixture.run(&["--output", "alias.js"]).status.success());
        assert_eq!(
            fs::read_to_string(fixture.0.path().join("prefs/privacy.toml")).unwrap(),
            original
        );
    }
}

#[test]
fn apply_installs_once_preserves_source_and_refreshes_generated_preferences() {
    let fixture = Fixture::new();
    let source = "// Documentation mentions #include aion.js\npref(\"upstream.setting\", true);";
    fixture.write("engine/browser/app/profile/firefox.js", source);
    fixture.write(
        "prefs/privacy.toml",
        "[[pref]]\nname = 'example'\nvalue = false\nlocked = true\n",
    );
    let result = fixture.run(&["--apply"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let firefox = fixture
        .0
        .path()
        .join("engine/browser/app/profile/firefox.js");
    let generated = fixture.0.path().join("engine/browser/app/profile/aion.js");
    let expected = format!("{source}\n\n#include aion.js\n");
    assert_eq!(fs::read_to_string(&firefox).unwrap(), expected);
    assert!(
        fs::read_to_string(&generated)
            .unwrap()
            .contains("pref(\"example\", false, locked);")
    );
    assert!(fixture.run(&["--apply"]).status.success());
    assert_eq!(fs::read_to_string(&firefox).unwrap(), expected);
    fixture.write(
        "prefs/privacy.toml",
        "[[pref]]\nname = 'example'\nvalue = true\nlocked = true\n",
    );
    assert!(fixture.run(&["--apply"]).status.success());
    assert!(
        fs::read_to_string(generated)
            .unwrap()
            .contains("pref(\"example\", true, locked);")
    );
    assert_eq!(fs::read_to_string(firefox).unwrap(), expected);
}

#[test]
fn apply_normalizes_duplicate_conditional_includes_and_preserves_crlf() {
    let fixture = Fixture::new();
    fixture.write(
        "checkout/browser/app/profile/firefox.js",
        "// Source\r\n#ifdef XP_WIN\r\n#include aion.js\r\n#endif\r\n#include aion.js\r\n",
    );
    fixture.write(
        "prefs/privacy.toml",
        "[[pref]]\nname = 'example'\nvalue = false\n",
    );
    let result = fixture.run(&["--apply", "--engine", "checkout"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(
            fixture
                .0
                .path()
                .join("checkout/browser/app/profile/firefox.js")
        )
        .unwrap(),
        "// Source\r\n#ifdef XP_WIN\r\n#endif\r\n\r\n#include aion.js\r\n"
    );
    assert!(!fixture.0.path().join("engine").exists());
}

#[test]
fn apply_failures_do_not_modify_checkout_or_create_missing_engine() {
    let fixture = Fixture::new();
    fixture.write(
        "prefs/privacy.toml",
        "[[pref]]\nname = 'example'\nvalue = false\n",
    );
    assert!(!fixture.run(&["--apply"]).status.success());
    assert!(!fixture.0.path().join("engine").exists());
    let source = "pref(\"upstream.setting\", true);\n";
    fixture.write("engine/browser/app/profile/firefox.js", source);
    fixture.write(
        "engine/browser/app/profile/aion.js",
        "previous generated output",
    );
    for arguments in [
        &["--apply", "--output", "custom.js"][..],
        &["--engine", "engine"][..],
    ] {
        assert!(!fixture.run(arguments).status.success());
    }
    fixture.write(
        "prefs/privacy.toml",
        "[[pref]]\nname = 'example'\nvalue = false\nlock = true\n",
    );
    assert!(!fixture.run(&["--apply"]).status.success());
    assert_eq!(
        fs::read_to_string(
            fixture
                .0
                .path()
                .join("engine/browser/app/profile/firefox.js")
        )
        .unwrap(),
        source
    );
    assert_eq!(
        fs::read_to_string(fixture.0.path().join("engine/browser/app/profile/aion.js")).unwrap(),
        "previous generated output"
    );
    assert!(!fixture.0.path().join("custom.js").exists());
}

#[cfg(unix)]
#[test]
fn partial_write_failure_preserves_previous_output_and_cleans_temporary_file() {
    let fixture = Fixture::new();
    fixture.write(
        "prefs/privacy.toml",
        &format!(
            "[[pref]]\nname = 'example'\nvalue = '{}'\n",
            "a".repeat(4096)
        ),
    );
    fixture.write("dist/prefs/aion.js", "previous output");
    // A file-size limit forces partial writes even when tests run as root.
    let result = Command::new("sh")
        .args([
            "-c",
            "trap '' XFSZ\nulimit -f 1 || exit 90\nexec \"$@\"",
            "aion-prefs-test",
            env!("CARGO_BIN_EXE_aion-prefs"),
        ])
        .current_dir(fixture.0.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_ne!(
        result.status.code(),
        Some(90),
        "could not set file-size limit"
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("aion-prefs:"));
    assert_eq!(
        fs::read_to_string(fixture.0.path().join("dist/prefs/aion.js")).unwrap(),
        "previous output"
    );
    assert_eq!(
        fs::read_dir(fixture.0.path().join("dist/prefs"))
            .unwrap()
            .count(),
        1
    );
}
