use super::{parse, render};

#[test]
fn preserves_string_types_and_default_attributes() {
    let prefs = parse("[[pref]]\nname = 'example.string'\nvalue = 'true'\n").unwrap();
    assert!(render(&prefs).ends_with("pref(\"example.string\", \"true\");\n"));
    let prefs = parse("[[pref]]\nname = 'example.string'\nvalue = '007'\n").unwrap();
    assert!(render(&prefs).ends_with("pref(\"example.string\", \"007\");\n"));
}

#[test]
fn escapes_strings_for_firefox_parser() {
    let prefs = parse(
        r#"
[[pref]]
name = 'example.escaped'
value = "a\"\\\n\r\t\b\fé🦊"

[[pref]]
name = 'example.multiline'
value = '''
C:\new\folder\
second line'''
"#,
    )
    .unwrap();
    let generated = render(&prefs);
    assert!(generated.contains(concat!(
        r#"pref("example.escaped", "a\"\\\n\r\u0009\u0008\u000Cé🦊");"#,
        "\n"
    )));
    assert!(generated.contains(concat!(
        r#"pref("example.multiline", "C:\\new\\folder\\\nsecond line");"#,
        "\n"
    )));
}

#[test]
fn renders_conditions_with_attributes_on_both_branches() {
    let prefs = parse(
        r#"
[[pref]]
name = 'example.platform'
value = true
else_value = false
condition = 'defined(XP_WIN)'
locked = true
"#,
    )
    .unwrap();
    assert!(render(&prefs).ends_with(
        r#"#if defined(XP_WIN)
pref("example.platform", true, locked);
#else
pref("example.platform", false, locked);
#endif
"#
    ));
    let prefs = parse("[[pref]]\nname = 'example.sticky'\nvalue = 3\nsticky = true\n").unwrap();
    assert!(render(&prefs).ends_with("pref(\"example.sticky\", 3, sticky);\n"));
    let prefs = parse("[[pref]]\nname = 'example.both'\nvalue = 3\nsticky = true\nlocked = true\n")
        .unwrap();
    assert!(render(&prefs).ends_with("pref(\"example.both\", 3, locked, sticky);\n"));
}

#[test]
fn enforces_integer_bounds_and_supported_types() {
    for value in ["-2147483648", "2147483647"] {
        assert!(
            parse(&format!(
                "[[pref]]\nname = 'example.int'\nvalue = {value}\n"
            ))
            .is_ok()
        );
    }
    for value in [
        "-2147483649",
        "2147483648",
        "1.5",
        "[]",
        "{}",
        "1979-05-27",
        r#""\u0000""#,
    ] {
        assert!(
            parse(&format!(
                "[[pref]]\nname = 'example.invalid'\nvalue = {value}\n"
            ))
            .is_err(),
            "accepted {value}"
        );
    }
}

#[test]
fn rejects_unknown_fields_and_invalid_combinations() {
    for (extra, expected_error) in [
        ("lock = true", "unknown field"),
        ("locked = 'true'", "invalid type"),
        ("else_value = false", "else_value requires condition"),
        ("condition = ''", "condition must"),
        (
            r#"condition = "defined(XP_WIN)\n#error broken""#,
            "condition must",
        ),
        (
            "condition = 'defined(XP_WIN)'\nelse_value = 42",
            "same type",
        ),
    ] {
        let error = parse(&format!(
            "[[pref]]\nname = 'example.invalid'\nvalue = true\n{extra}\n"
        ))
        .unwrap_err()
        .to_string();
        assert!(
            error.contains(expected_error),
            "{extra}: expected {expected_error}, got {error}"
        );
    }
    for name in ["", "invalid name", "invalid\tname"] {
        assert!(parse(&format!("[[pref]]\nname = '{name}'\nvalue = true\n")).is_err());
    }
    assert!(parse("unknown = true\n[[pref]]\nname = 'example'\nvalue = true\n").is_err());
    assert!(parse("[[pref]]\nname = 'missing.value'\n").is_err());
}
