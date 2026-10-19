# Aion development plan

Aion starts from upstream Firefox and targets Linux x86_64. The development workflow must fit a 16 GB RAM machine without a recurring cloud subscription. Browser interface and agent integration work come first; native engine changes follow only when their benefit justifies the additional build and maintenance work.

## Foundation

Maintain Aion features as small, reviewable changes against Firefox. Keep Aion implementation in its own modules where possible, with narrow hooks into upstream code. Record the upstream revision used by every successful build.

Use an upstream revision with matching downloadable artifacts for the initial prototype. Validate artifact availability before choosing a particular release or ESR revision; artifact builds depend on compatible native binaries. A release branch, security update policy, and packaging workflow must be selected before public distribution.

[Mozilla's artifact-build documentation](https://firefox-source-docs.mozilla.org/contributing/build/artifact_builds.html) explains the supported changes and source-to-binary compatibility constraints. [Zen](https://github.com/zen-browser/desktop) and [Floorp](https://github.com/Floorp-Projects/Floorp) provide references for Firefox customization. They are references rather than Aion's upstream base.

## Knowledge and tools

| Area | Initial focus |
| --- | --- |
| Browser frontend | JavaScript modules, HTML/XHTML, CSS, browser events, accessibility, and keyboard interaction |
| Firefox integration | Privileged browser modules, preferences, window and tab lifecycles, and the boundary between browser UI and untrusted pages |
| Development tools | Git, Python, `mach`, `mozconfig`, `moz.build`, browser debugging, and targeted tests |
| Agent integration | WebExtensions, native messaging, WebDriver BiDi, explicit permissions, and structured messages |
| Native implementation | C++ and Rust when modifying compiled browser components |
| Maintenance | Rebasing changes, reviewing upstream updates, packaging, and testing upgrade behavior |

Use Mozilla's bootstrap tooling to provision the toolchain required by the selected source revision. Avoid independently selecting compiler versions or combining instructions from unrelated Firefox forks.

## Architecture

Firefox separates its browser interface and privileged services from web content processes. Aion's initial changes should stay in the browser frontend and narrowly scoped services.

| Layer | Role in Aion |
| --- | --- |
| Browser frontend | Toolbar, tabs, settings, command interface, and agent panel |
| Privileged browser services | Preferences, permissions, navigation, and mediation of agent requests |
| Gecko and native components | Existing rendering, networking, storage, and process isolation implementation |
| WebExtensions | A possible first implementation of page interaction and agent integration |
| Optional local companion | Connect an external agent or model provider through a defined interface |

Firefox supports [WebDriver BiDi and Marionette](https://firefox-source-docs.mozilla.org/remote/index.html) for automation. [Native messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging) connects an extension to a separately installed local application. Evaluate these mechanisms before adding a custom native browser API.

## Local system requirements

Mozilla's [Linux setup guide](https://firefox-source-docs.mozilla.org/setup/linux_build.html) lists 4 GB RAM minimum, 8 GB recommended, and at least 30 GB free disk space. These are baseline requirements, rather than a prediction of the size of Aion's checkout, caches, tests, and release output.

The current machine has 16 GB RAM and ample SSD capacity for the initial workflow. Reserve roughly 100 GB as a planning allowance for a single development checkout and its supporting files, increasing it when actual usage warrants it. This allowance is not a measured requirement.

Artifact builds avoid compiling the native engine locally. Native builds can still require reduced parallelism and SSD-backed swap. The machine currently has 4 GB of zram swap; compressed RAM swap does not provide the same headroom as additional disk-backed swap. Decide any swap changes when a native build becomes necessary.

Arch Linux is a reasonable starting host. Mozilla maintains Ubuntu bootstrapping directly, while support for other distributions relies more on community maintenance. Investigate failures against the selected revision before changing the host environment. A container can isolate dependencies but does not increase available RAM.

## Artifact development workflow

1. Follow the current Mozilla Linux bootstrap instructions and select Firefox for Desktop Artifact Mode.
2. Confirm a successful unmodified build and launch before applying Aion changes.
3. Record the source revision and build configuration.
4. Make one small interface change and verify that the artifact workflow incorporates it.
5. Preserve the checkout, artifact cache, and object directory for subsequent work.
6. Run targeted tests and browser smoke checks after each meaningful change.

Mozilla documents this artifact configuration in `mozconfig` inside the Firefox source checkout:

```sh
ac_add_options --enable-artifact-builds
mk_add_options MOZ_OBJDIR=./objdir-frontend
```

After bootstrapping, the normal build and launch commands are:

```sh
./mach build
./mach run
```

For supported local frontend changes, use:

```sh
./mach build faster
```

Use a development profile separate from the user's everyday Firefox profile. Keep existing build output unless the toolchain requires a clean build.

Artifact builds support JavaScript, HTML/XHTML, CSS, localization, and packaged resources. They do not incorporate changes to C, C++, or Rust, and have restrictions on WebIDL and build definitions. Compiled privacy changes or new native APIs require newly built native artifacts. Artifact builds must not be presented as a way to ship arbitrary engine modifications without compiling them.

Mozilla's [Marionette build guide](https://firefox-source-docs.mozilla.org/remote/marionette/Building.html) gives an example of artifact builds taking about one to two minutes with a fast internet connection. Initial checkout and downloads take longer; the Linux setup guide suggests allowing an hour or two. Actual Aion build times must be measured on this machine. No local build benchmark has been performed.

## Privacy and agent behavior

Begin with an inventory of background requests and service integrations. For each change, record the feature, upstream default, Aion behavior, expected compatibility effect, and verification method. Validate behavior with fresh profiles and network observations rather than relying only on preference names or source searches.

Treat telemetry, crash reporting, search suggestions, remote configuration, account integration, and model-provider connections as separate decisions. Preserve browser security mechanisms and define how security-related data and browser updates remain current.

Agent support should be optional. Start with a narrow workflow such as reading a user-selected page and proposing an action. Scope access to the selected tab or origin, show when an agent is active, and allow immediate revocation. Keep page content separate from trusted instructions. Require explicit approval for sensitive actions such as submitting forms or sending page content to a remote provider.

The first agent prototype can use a WebExtension plus a local companion. Incorporate it into the browser interface once its behavior and permission model are useful. Check Firefox compatibility for every automation library or extension dependency; Chromium-specific integrations may require adaptation.

## Milestones

| Milestone | Completion condition |
| --- | --- |
| Establish the build | Unmodified artifact build launches locally with an isolated profile; revision and timings recorded |
| Demonstrate customization | One visible Aion interface change works through an incremental artifact build |
| Build the minimal interface | A small, keyboard-accessible interface supports normal browsing and settings |
| Establish privacy defaults | Each selected privacy change has a documented rationale and a behavior check |
| Demonstrate agent interaction | A user can grant access to one page, inspect the proposed action, and revoke access |
| Rehearse an upstream update | Aion changes apply to a newer upstream revision and pass targeted checks |
| Prepare a release | Packaging, branding, notices, security updates, and upgrade tests are defined and validated |

Advance by working milestones rather than assuming a fixed launch date. Native features should have a separate build experiment before being added to the release scope.

## Cost and infrastructure

Start locally using Mozilla's downloadable artifacts. This has no required build-service subscription; internet access, electricity, and any model-provider usage remain separate costs.

Use free CI for lightweight checks and tests that fit its limits. GitHub's standard Linux runners advertise only 14 GB SSD storage, so they are not a dependable default for a complete Firefox checkout and native build. See [runner specifications](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

If a native change later exceeds local resources, compare a one-off cloud build with a remote machine that preserves the checkout and outputs. Select architecture, RAM, storage, and region from a measured build requirement. Trial credits can fund experiments but are temporary; sponsorship should not be assumed.

Before public distribution, validate how compatible native artifacts will be produced or obtained for every release. Daily frontend development and reproducible release production are separate milestones.
