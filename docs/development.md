# Development

Aion is in early development. The current setup builds upstream Firefox for frontend work. Aion-specific features and patch tooling are still to be added.

## Prerequisites

Install Git, Python 3.10 or newer, and curl. Follow Mozilla's requirements for your platform before running bootstrap:

- [Linux](https://firefox-source-docs.mozilla.org/setup/linux_build.html)
- [macOS](https://firefox-source-docs.mozilla.org/setup/macos_build.html)
- [Windows](https://firefox-source-docs.mozilla.org/setup/windows_build.html)

Mozilla's bootstrap script installs the remaining development dependencies. The commands below use a Unix shell; on Windows, use the environment described in Mozilla's guide.

## Setup

Clone the repository:

```sh
git clone --branch main https://github.com/Ethics03/aion.git
cd aion
```

Download and run Mozilla's bootstrap script:

```sh
curl -fL --retry 3 \
  https://raw.githubusercontent.com/mozilla-firefox/firefox/refs/heads/main/python/mozboot/bin/bootstrap.py \
  -o /tmp/aion-firefox-bootstrap.py &&
python3 /tmp/aion-firefox-bootstrap.py \
  --application-choice browser_artifact_mode
```

When asked for the checkout destination, enter `engine`. Follow the prompts to install dependencies and create the suggested build configuration.

If `engine/` already contains a Firefox checkout, run this inside it instead:

```sh
./mach bootstrap --application-choice browser_artifact_mode
```

Check that `engine/mozconfig` includes:

```sh
ac_add_options --enable-artifact-builds
```

Preserve any other settings bootstrap generated.

## Build and run

From the Aion repository root:

```sh
cargo run --locked -p aion-prefs -- --apply
cd engine
./mach build
./mach run
```

For subsequent preference, JavaScript, CSS, markup, and packaged-resource changes, run from the repository root:

```sh
cargo run --locked -p aion-prefs -- --apply
cd engine
./mach build faster
./mach run
```

Artifact builds use Mozilla's precompiled native components. Changes to Firefox's C++, Rust, WebIDL, or some build definitions require a different build workflow. See [artifact build support and restrictions](https://firefox-source-docs.mozilla.org/contributing/build/artifact_builds.html).

## Rust tooling

Aion's development tools use a separate Cargo workspace. Mozilla's bootstrap installs Rust and Cargo; these tools compile independently of Firefox and work alongside artifact builds.

From the Aion repository root:

```sh
cargo check --workspace
cargo run --locked -p aion-prefs
```

`aion-prefs` reads TOML files recursively from `prefs/`, validates them, and writes Firefox preference statements to `dist/prefs/aion.js`. Files and preference names are sorted for repeatable output. It validates all input and writes a complete temporary file before atomically replacing existing output, so a validation or write error preserves the previous file. Input directories must contain regular files and directories; symlinks are rejected.

Run from the repository root, or pass explicit paths:

```sh
cargo run --locked -p aion-prefs -- --input prefs --output dist/prefs/aion.js
cargo test --workspace --locked
```

The tool uses `serde` for typed configuration, `toml` for parsing, `anyhow` for contextual errors, and `tempfile` for safely replacing generated output. Output paths cannot have a `.toml` extension or resolve to a TOML file. Paths starting with `-` need a `./` prefix so they are distinguishable from options. Generation does not require a Firefox checkout. Commit the root `Cargo.lock` when dependencies change; compiled tools go into `target/` and generated preferences into `dist/`, both ignored by Git.

To install the generated preferences in a Firefox checkout:

```sh
cargo run --locked -p aion-prefs -- --apply
```

Apply mode writes `engine/browser/app/profile/aion.js` and ensures one unconditional `#include aion.js` at the end of Firefox's `firefox.js`. Repeated runs update the generated preferences without duplicating the include. Other Firefox source lines and their line endings are preserved. Validation happens before any checkout changes, and the generated file is installed before adding its include. Each file is replaced atomically; the pair is not a filesystem transaction.

Use `--apply --engine /path/to/firefox` for a different checkout. `--output` is for standalone generation and cannot be combined with `--apply`. After applying, rebuild and restart Firefox. In `about:config`, check that `datareporting.healthreport.uploadEnabled`, `datareporting.usage.uploadEnabled`, and `nimbus.rollouts.enabled` are false and locked. `app.normandy.enabled` defaults to false without a lock.

Each preference is a `[[pref]]` entry:

```toml
# Disable usage reporting.
[[pref]]
name = "datareporting.usage.uploadEnabled"
value = false
locked = true
```

| Field | Meaning |
| --- | --- |
| `name` | Required, unique preference name |
| `value` | Required boolean, string, or signed 32-bit integer |
| `locked` | Optional, defaults to `false`; prevents user values from taking effect |
| `sticky` | Optional, defaults to `false`; preserves an explicitly set user value even when it matches the default |
| `condition` | Optional, single-line Firefox preprocessor expression, such as `defined(XP_WIN)` |
| `else_value` | Optional fallback for a false condition; requires `condition` and the same type as `value` |

Quoted values retain their type: `"true"` and `"007"` are strings. Multiline TOML strings are supported. Unknown fields, missing fields, duplicate names, unsupported value types, NUL characters, and integers outside Firefox's range cause errors. Conditional expressions are evaluated by Firefox's preprocessor during the browser build.

In the Rust source, `main.rs` defines the default paths and command options. `prefs.rs` defines `PreferenceFile`, `Preference`, and the typed `PreferenceValue` enum. Each preference validates its own field combinations. `name` and `value` have no defaults, so an omitted setting cannot silently turn into a different value.

Keep Aion's tests under `src/aion/tests/`, grouped by feature. Preference unit tests live in `src/aion/tests/prefs/unit.rs` and CLI and filesystem regressions in `src/aion/tests/prefs/cli.rs`. The Rust test-module declaration and Cargo test target point to these files; unit tests are included only for test builds. Run them with `cargo test --workspace --locked`.

Standalone generation does not change the browser; apply mode installs the settings for the next build. The settings in `prefs/privacy.toml` cover reporting controls, selected legacy pings, and study enrollment. Global Privacy Control is enabled, crash-report submission defaults to off, and the telemetry server is `data:,`. These settings do not remove compiled components or establish that all reporting paths have been disabled.

`prefs/newtab.toml` defaults stories, shortcuts, widgets, Contile, and weather availability to off. Sponsored content defaults to off without locks, and its settings controls are hidden. Widget and ad-provider feeds require their related availability and feed preferences before use. New-tab reporting and promotional messages remain locked off. The search box stays available. Enabling remote content can make requests to its providers.

Some preferences, including `toolkit.telemetry.enabled`, are also assigned by native startup code. Runtime verification must check their final effective values.

## Project structure

```text
aion/
├── README.md
├── Cargo.toml        # Workspace for Aion's Rust tools
├── Cargo.lock        # Pinned tool dependencies
├── tools/
│   └── prefs/        # Rust preference generator
├── src/
│   ├── aion/         # Aion features and interface
│   │   └── tests/    # Aion tests, grouped by feature
│   └── browser/      # Patches to Firefox browser files
├── prefs/            # Default browser preferences
├── configs/          # Shared build configuration
├── scripts/          # Setup and development helpers
├── docs/             # Development documentation
└── engine/           # Local Firefox checkout, excluded from Aion's Git history
```

The `src/browser/`, `configs/`, and `scripts/` directories currently contain placeholders. The preference tool installs its own build include; other Aion-specific source integration is still to be added.

Within `engine/`, the main frontend directories are:

| Directory | Purpose |
| --- | --- |
| `browser/base/content/` | Browser window markup and scripts |
| `browser/themes/` | Interface styles and assets |
| `browser/components/` | Features such as tabs, the address bar, and settings |
| `browser/locales/` | Interface strings and translations |

## Making changes

Keep changes small and verify them in the running browser. Run relevant checks and tests for the behavior you change.

Keep Rust imports at module scope with explicit names, grouped into standard library, dependencies, and project imports. Use conditional imports for platform-specific code. Avoid function-local imports and wildcard imports, including in tests.

Development tools use `anyhow::Result` and attach file or operation context while preserving underlying errors. CLI diagnostics print the error chain so the original parse or filesystem failure remains visible.

Files inside `engine/` are ignored by the Aion repository. The tracked preference tool reproduces its generated file and include through `--apply`. Other Firefox modifications must be preserved as tracked patches, and new Aion files must be tracked separately, before they can be shared. Patch import/export tooling and a pinned Firefox revision have not been added yet.
