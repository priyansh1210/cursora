# 12: Microsoft Store (design)

Status: **Draft** (2026-10-26). Implements [requirements.md](requirements.md). The Store version is the same `cursora.exe` and engine as the .msi, put into an MSIX package with Microsoft's `makeappx`. At startup Cursora asks Windows whether it runs from a package. If it does, the updater and Run as administrator are off, "Start with system" uses the package's startup task, and About shows where Windows really keeps its files. The Report button and the privacy page are in both versions.

## Overview
```
v* tag ─→ release job, Windows: tauri-action builds target/release/cursora.exe + llama\ (as today)
       ─→ scripts/msix.mjs: a folder with cursora.exe, llama\, Assets\ (Tauri's Store logos) and AppxManifest.xml,
          its identity from the repo variables STORE_IDENTITY_NAME, STORE_PUBLISHER, STORE_PUBLISHER_NAME (STO-1.2)
       ─→ makeappx pack → Cursora_<version>_x64.msix (unsigned: the Store signs it)
       ─→ a copy signed with a throwaway test certificate, plus that certificate (.cer), for trying it first (STO-1.3)
       ─→ both kept as an artifact of the run (private), not in the public release (STO-1.1)
developer ─→ downloads the artifact, uploads the unsigned .msix in Partner Center, submits
Store     ─→ certifies, signs, publishes; then updates people's copies itself
```

## Components
| File | Role |
|---|---|
| `scripts/msix.mjs` (new) | Writes `AppxManifest.xml` from the template, the app's version and the identity, gathers the files, runs `makeappx`. The same script works on the laptop, which has the Windows SDK. |
| `scripts/msix/AppxManifest.xml` (new) | The manifest template (below). |
| `.github/workflows/ci.yml` | The release job's Windows run: `node scripts/msix.mjs`, the test signature, `actions/upload-artifact`. It's skipped with a notice while the identity variables don't exist yet. |
| `src-tauri/src/store.rs` (new) | `packaged() -> bool` (`GetCurrentPackageFullName`; cached); `startup(enable)` through `Windows.ApplicationModel.StartupTask`; `data_dirs()`: the redirected folders. |
| `src-tauri/src/update.rs` | `start()` and `update_check` do nothing when packaged (STO-2.1). |
| `src-tauri/src/lib.rs` | `sync_autostart` uses `store::startup` when packaged (STO-2.2); `app_info` gets `store: bool`, and the folders from `store::data_dirs()` (STO-2.5); `open_folder` too. |
| `src-tauri/src/admin.rs` | `available()` is false when packaged: the package lives in `C:\Program Files\WindowsApps`, which `in_program_files()` would otherwise accept (STO-2.3). |
| `src/Settings.tsx` | General: no "Check for updates automatically" in the Store version; Run as administrator's note says it needs the GitHub version. About: "Updates come from the Microsoft Store" instead of Check for updates; a **Privacy policy** link. |
| `src/Chat.tsx`, `src/report.ts` (new) | **Report** on every answer (STO-3.1). |
| `cursora-releases`: `privacy.md` | The privacy policy, on GitHub Pages (STO-3.2). |
| `specs/12-microsoft-store/listing.md` (new) | The Store listing's texts and the certification notes, ready to paste (STO-3.3). |

## The manifest
```xml
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="…/uap/windows10" xmlns:uap3="…/uap/windows10/3" xmlns:desktop="…/desktop/windows10"
         xmlns:rescap="…/foundation/windows10/restrictedcapabilities">
  <Identity Name="{STORE_IDENTITY_NAME}" Publisher="{STORE_PUBLISHER}" Version="{X.Y.Z}.0" ProcessorArchitecture="x64"/>
  <Properties><DisplayName>Cursora</DisplayName><PublisherDisplayName>{STORE_PUBLISHER_NAME}</PublisherDisplayName><Logo>Assets\StoreLogo.png</Logo></Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.26100.0"/>
    <!-- llama-server needs the Visual C++ runtime; the Store installs this framework with Cursora -->
    <PackageDependency Name="Microsoft.VCLibs.140.00.UWPDesktop" MinVersion="14.0.30035.0" Publisher="CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US"/>
  </Dependencies>
  <Resources><Resource Language="en-us"/></Resources>
  <Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities>
  <Applications>
    <Application Id="Cursora" Executable="cursora.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="Cursora" Description="Your AI companion, right at your cursor"
          BackgroundColor="transparent" Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png"/>
      <Extensions>
        <desktop:Extension Category="windows.startupTask" Executable="cursora.exe" EntryPoint="Windows.FullTrustApplication">
          <desktop:StartupTask TaskId="CursoraStartup" Enabled="false" DisplayName="Cursora"/>
        </desktop:Extension>
        <uap3:Extension Category="windows.appExecutionAlias" Executable="cursora.exe" EntryPoint="Windows.FullTrustApplication">
          <uap3:AppExecutionAlias><desktop:ExecutionAlias Alias="cursora.exe"/></uap3:AppExecutionAlias>
        </uap3:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>
```
The startup task starts off (`Enabled="false"`). Cursora turns it on when "Start with system" is on, which the user chose in setup. That's the consent policy 10.2.8 asks for.

## Key mechanics

### Packaged or not
`store::packaged()` calls `GetCurrentPackageFullName`. It returns `APPMODEL_ERROR_NO_PACKAGE` for the .msi and dev builds. The answer is cached in a `OnceLock`. Linux: always false.

### Updates (STO-2.1)
`update::start` returns at once when packaged, so the Store version never contacts `cursora-releases`. `update_check` answers "Updates come from the Microsoft Store." `AppInfo.store` hides the setting and the button.

### Start with system (STO-2.2)
When packaged, `sync_autostart` calls `StartupTask::GetAsync("CursoraStartup")`, then `RequestEnableAsync()` or `Disable()`, on a background thread (WinRT, `windows` features `ApplicationModel` and `Foundation`). The Run key isn't touched: Windows keeps a packaged app's registry writes to itself, so it would never take effect. If the user turned Cursora off in Task Manager's Startup apps, the state is `DisabledByUser`. Settings then shows the box unticked, with "Turned off in Windows' Startup apps", since only the user can turn it back on there.

### Run as administrator (STO-2.3)
`admin::available()` is false when packaged. Settings shows the row greyed out: "Only in the version from GitHub (cursora-releases)."

### Command line (STO-2.4)
The alias puts `cursora.exe` on the path for the user. `cursora --ask` reaches the running copy through the single-instance plugin, as now.

### Folders (STO-2.5)
Windows redirects a packaged full-trust app's writes to AppData into `%LOCALAPPDATA%\Packages\<family name>\LocalCache\Roaming` and `…\Local`. Cursora's own reads and writes go through that redirection unchanged. But Explorer would show the empty, un-redirected folder, so `store::data_dirs()` builds the real paths from `GetCurrentPackageFamilyName`, and About shows and opens those.

### Report (STO-3.1)
A **Report** link in each answer's actions (next to Copy, Regenerate, Save as skill) opens a small panel:
- the checkbox "Include the question and answer" (unticked)
- **Open email**
- Cancel

`report.ts` builds a `mailto:` link to `REPORT_EMAIL`. It's the one place the address is written, so it can change with a release. The subject is "Cursora report: <model>"; the body has the Cursora version, the model and provider, and "What's wrong:", then, if ticked, the question and the answer. The link is kept under 1,800 characters, cutting the answer with "…", since Windows rejects longer `mailto:` links. It opens through the opener plugin (`mailto:*` is already allowed in the bubble's capabilities). Nothing is sent until the user presses Send in their own email app.

### Privacy policy (STO-3.2)
`privacy.md` in `cursora-releases`, with GitHub Pages turned on for that repo (main branch, root), so it's at `https://priyansh1210.github.io/cursora-releases/privacy`. About links to it. Claude drafts it from what Cursora does, and the user approves it:
- **what Cursora reads:** the selection, files and snips, only when asked
- **what it stores:** settings, history, keys in the keychain, logs, all on the computer
- **what it sends:** the question and context, only to the AI provider the user chose, after redaction; and the update check, in the GitHub version only
- **what the developer receives:** nothing, except what a user emails through Report
- **the controls:** local-only, the blocklist, history off, deleting history

### The package in CI (STO-1.1 to 1.3)
In the release job's Windows run, after tauri-action:
1. `node scripts/msix.mjs --out dist-msix` gathers `src-tauri/target/release/cursora.exe`, `src-tauri/target/release/llama\` and `src-tauri/icons/{Square44x44Logo,Square150x150Logo,StoreLogo}.png` into `Assets\`. It writes the manifest and runs `makeappx pack /o /d <folder> /p Cursora_<version>_x64.msix`, finding `makeappx` under `C:\Program Files (x86)\Windows Kits\10\bin\<newest>\x64`.
2. A throwaway certificate whose subject is `STORE_PUBLISHER` (`New-SelfSignedCertificate`) signs a copy, `Cursora_<version>_x64_test.msix`, with `signtool`, and is exported as `Cursora_test.cer`.
3. `actions/upload-artifact` keeps the three files with the run (`msix-<version>`) for 30 days.

To try it, the developer installs `Cursora_test.cer` into "Trusted People" once (as administrator), then opens the test package. The unsigned one goes to Partner Center.

## Changes to earlier specs
- **APP-3 (autostart, Run as administrator):** packaged branches, as above.
- **UPD:** off when packaged. The release checklist gets a step after publishing on GitHub: download the `msix-<version>` artifact and submit it in Partner Center.
- **BUB-6 (answer actions):** Report joins Copy, Regenerate and Save as skill.

## Dependencies
- `windows` features `Win32_Storage_Packaging_Appx` (package identity), and `ApplicationModel` and `Foundation` (the startup task, through WinRT). Same crate, more features.
- Nothing new in npm. The Windows SDK (`makeappx`, `signtool`) is already on GitHub's Windows runners and on the laptop.

## Decisions
1. **Our own MSIX** from the same build, not Tauri's .msi in the Store: an .msi submission needs our own paid signature, while an MSIX is signed by Microsoft (requirements). Rejected: converting the .msi with the MSIX Packaging Tool (an interactive tool, harder in CI).
2. **One `cursora.exe` for both versions**, behaving by `packaged()`, not a separate Store build: one build, one thing to test. Rejected: a Cargo feature for the Store (two builds, and the .msi would have to be built twice).
3. **The package is an artifact of the private run**, not a public release file (STO-1.1 amended).
4. **The VC++ runtime from Microsoft's framework package** (`Microsoft.VCLibs.140.00.UWPDesktop`): the Store installs it. The .msi has the same need, logged as issue 50 and fixed separately.
5. **Report through the user's own email app** (`mailto:`): no server of ours, nothing sent without the user seeing it. Rejected: a web form (a server to run, and data passing through it).
6. **The startup task starts off**, and Cursora turns it on from the user's "Start with system" choice (policy 10.2.8's consent).

## Known limits
- `shortcut:` the identity variables must be set by hand once the Partner Center app exists. Until then the MSIX step is skipped with a notice.
- `shortcut:` the redirected folder paths follow Microsoft's documented layout. Check them on the first real install (STO-2.5).
- `shortcut:` the developer submits each version in Partner Center by hand. The Store's submission API could do it later.

## Test strategy
| Criterion | How |
|---|---|
| STO-1.2 | `scripts/msix.test.mjs`: the manifest from the template has the identity, `X.Y.Z.0`, `runFullTrust`, the startup task (off), the alias and the VCLibs dependency, and is well-formed XML. |
| STO-2.1 | Rust test of the pure rule `update::checks_allowed(packaged, setting)`: never when packaged. |
| Manual | STO-1.1, 1.3: a tag's run has the artifact, and the test package installs on the laptop. STO-2.2 to 2.6 on the installed test package: startup task, no Run as administrator, `cursora --ask`, settings, history, keys, folders, hotkeys, capture, engine, snip, tray. STO-3.1: Report opens the email with and without the Q&A. STO-3.2: the page is live and linked. STO-3.3: the listing is accepted. |
