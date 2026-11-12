# Contributing to Cursora

Thanks for helping. Bug reports, hands-on checks and code are all welcome.

## Good places to start
- **Hands-on checks.** Many [open issues](https://github.com/priyansh1210/cursora/issues?q=is%3Aopen+label%3A%22needs+testing%22) only need someone with the right setup to try something: a real Linux desktop, two monitors, a screen reader, an app we don't have. Try it and tell us what happened. That counts as a contribution.
- **Issues labelled** [`good first issue`](https://github.com/priyansh1210/cursora/labels/good%20first%20issue) or [`help wanted`](https://github.com/priyansh1210/cursora/labels/help%20wanted).
- **Bugs.** Open an issue with the bug report template. Say which app you selected text in, and which model you used.

## How Cursora is built
Cursora is built **spec-first**. Read the [constitution](specs/00-constitution.md) and the [specs](specs/README.md) before changing anything.
- A behavior change goes into its spec (`requirements.md`, then `design.md`) before the code.
- Every acceptance criterion has an ID (`SEL-3.1`). Each (auto) criterion needs a test named after it: `fn sel_3_1_what_it_checks()` in Rust, `it('SEL-3.1 …')` in TypeScript.
- Every problem found goes into [specs/issues.md](specs/issues.md), and moves to Resolved with its fix.
- Keep changes small. Mark a known limit in the code with `// shortcut: <the limit>, <when to upgrade>`.

## Setup and checks
See [Build from source](README.md#build-from-source) for what to install. Then:

```
npm install
npm run trace        # every (auto) spec criterion has a test
npm test             # JS tests
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev    # run the app
```

CI runs the same checks on Windows and Ubuntu for every pull request.

## Pull requests
- One topic per pull request, from a branch of your fork.
- Say which spec and criteria it touches, how you checked it, and what you couldn't check.
- Keep the user-facing wording plain and short, and use no emoji in the app's interface.

## Licensing of contributions
Cursora is licensed under the [GNU General Public License v3.0 or later](LICENSE). By sending a contribution, you agree that it is licensed under the same license. You also allow the maintainer to offer it under other license terms, for example in a paid edition, so that the project can keep going.

## Conduct
Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md).
