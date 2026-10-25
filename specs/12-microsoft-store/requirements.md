# 12: Microsoft Store (STO)

## Scope
- **In:** an MSIX package of Cursora, built by CI for each release and published in the Microsoft Store under the developer's free individual account. Microsoft signs it, so people who install from the Store see no "Windows protected your PC" warning. It also covers what Cursora must do differently when it runs as a Store app, and what the Store's policies require.
- **Out:**
  - Linux, which keeps its GitHub packages
  - macOS and Xbox
  - Paid listings and in-app purchases
  - Uploading to the Store automatically: the developer uploads each package in Partner Center
  - A paid code-signing certificate for the GitHub .msi (UPD stays as it is)

Why the Store: code signing outside it costs about $270–310 a year for an individual in India, and Azure Artifact Signing isn't open to individuals there. In the Store it's free, and Microsoft signs the package after certification (decided by the user on 2026-10-26). Tauri builds only .msi and .exe installers, and submitting those needs our own paid signature. So the free route is an MSIX package, which this spec builds.

## Requirements

### STO-1 The package
User story: As the developer, I want each release to come with a Store package, built the same way every time.
- STO-1.1 (manual) WHEN a `v*` tag is built THE SYSTEM (CI) SHALL also build an x64 MSIX package of Cursora with the built-in engine, and attach it to the release draft. The developer uploads it to Partner Center for certification.
- STO-1.2 (auto) THE package manifest SHALL declare:
  - the package identity and publisher that Partner Center assigns
  - the version, from `tauri.conf.json` as `X.Y.Z.0`
  - full trust (`runFullTrust`)
  - a startup task (STO-2.2)
  - the command alias `cursora.exe` (STO-2.4)
- STO-1.3 (manual) THE developer SHALL be able to install the package on the laptop before uploading it, with a test signature, to check it.

### STO-2 Running as a Store app
User story: As a user who installed Cursora from the Store, I want it to work like the downloaded version, with the Store keeping it up to date.
- STO-2.1 (auto) WHEN Cursora runs from the Store package THE SYSTEM SHALL NOT check for, download or install updates itself; the Store does (policy 10.2.2). Settings SHALL NOT show "Check for updates automatically" or About's **Check for updates**; About says "Updates come from the Microsoft Store."
- STO-2.2 (manual) "Start with system" SHALL use the package's startup task, which is what Windows' Startup apps list shows, instead of the Run key.
- STO-2.3 (manual) "Run as administrator" SHALL NOT be offered in the Store version; General says it needs the version from GitHub.
- STO-2.4 (manual) `cursora --ask`, `--snip` and `--settings` SHALL work from a command prompt, through the alias.
- STO-2.5 (manual) Settings, history and API keys SHALL work as in the .msi version. About's folder buttons SHALL open the folders where Windows actually keeps the Store version's files.
- STO-2.6 (manual) Everything else SHALL behave as in the .msi version: the hotkeys, reading selections in other apps, the built-in engine, cloud providers, snip, the tray and the privacy guard.

### STO-3 Store policies
User story: As the developer, I want the listing to pass certification and stay in the Store.
- STO-3.1 (manual) Every answer SHALL offer **Report** (policy 11.16). It opens the user's email app with a message to the developer's address, already filled in: the model's name, the Cursora version, and an empty space to describe the problem. The question and answer are attached only when the user ticks "Include the question and answer". Nothing is sent without the user pressing Send in their email app.
- STO-3.2 (manual) A public privacy policy page SHALL describe what Cursora reads, stores and sends, and to whom: nothing to the developer; to the AI provider the user chose; locally, the history. It is linked in Partner Center and from About (policy 10.5.1).
- STO-3.3 (manual) THE Store listing SHALL say first that Cursora needs an AI model, either on the computer or the user's own API key (policy 10.2.4), and that it shows AI-generated answers (policy 11.16). It also has the screenshots, the icon, the category (Productivity) and the age rating. Live generative AI is ticked in Partner Center.

### STO-4 Releases
- STO-4.1 (manual) Each version SHALL ship in both places with the same number: GitHub (.msi with Cursora's own updater, .deb, .rpm, AppImage) and the Store (MSIX). The release checklist (UPD design) gains the upload step.

## Decisions (2026-10-26, the user)
1. **The GitHub .msi stays**, next to the Store, which is the main link: the .msi is for people who don't use the Store, and it's the only version with Run as administrator (STO-2.3). It stays unsigned, with the warning.
2. **The report address (STO-3.1) is a new mailbox just for Cursora**, which the developer creates. It may later move to an address on Cursora's own domain, so it lives in one place in the code, and changes with a release.
3. **The privacy policy (STO-3.2) is a page in the public `cursora-releases` repo, through GitHub Pages.** Claude drafts it from what Cursora actually does, for the developer's approval.

## Open questions
1. **The Store name:** "Cursora" must be reserved in Partner Center when the developer account exists; if it's taken, another one.

## Known risk
Cursora reads selected text in other apps (UI Automation) and sends Ctrl+C to copy it. Policy 10.2.8 forbids using accessibility interfaces in unsupported ways to change Windows settings. Cursora doesn't change any settings, but a reviewer may ask, so the certification notes explain it up front. Each Store update also waits for certification, which takes up to a few days.
