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
cd engine
./mach build
./mach run
```

For subsequent JavaScript, CSS, markup, and packaged-resource changes, run from `engine/`:

```sh
./mach build faster
./mach run
```

Artifact builds use Mozilla's precompiled native components. Changes to C++, Rust, WebIDL, or some build definitions require a different build workflow. See [artifact build support and restrictions](https://firefox-source-docs.mozilla.org/contributing/build/artifact_builds.html).

## Project structure

```text
aion/
├── README.md
├── src/
│   ├── aion/         # Aion features and interface
│   └── browser/      # Patches to Firefox browser files
├── prefs/            # Default browser preferences
├── configs/          # Shared build configuration
├── scripts/          # Setup and development helpers
├── docs/             # Development documentation
└── engine/           # Local Firefox checkout, excluded from Aion's Git history
```

The `src/`, `prefs/`, `configs/`, and `scripts/` directories currently contain placeholders. They do not affect the Firefox build until the corresponding implementation and tooling are added.

Within `engine/`, the main frontend directories are:

| Directory | Purpose |
| --- | --- |
| `browser/base/content/` | Browser window markup and scripts |
| `browser/themes/` | Interface styles and assets |
| `browser/components/` | Features such as tabs, the address bar, and settings |
| `browser/locales/` | Interface strings and translations |

## Making changes

Keep changes small and verify them in the running browser. Run relevant checks and tests for the behavior you change.

Files inside `engine/` are ignored by the Aion repository. Firefox modifications must be preserved as tracked patches, and new Aion files must be tracked separately, before they can be shared. Patch import/export tooling and a pinned Firefox revision have not been added yet.
