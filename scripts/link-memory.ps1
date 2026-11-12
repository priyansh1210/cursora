# Maintainer only: clones the memory branch of the private cursora-mine repo into memory\ (this repo ignores it) and points Claude Code's memory
# folder for this clone at it, so every dev PC shares one memory. Run once per PC: powershell -File scripts\link-memory.ps1
# shortcut: Windows only; a Claude Code running inside WSL would need a symlink instead, if we ever run it there.
param([string]$From = 'https://github.com/priyansh1210/cursora-mine.git', [string]$Branch = 'memory')
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..").Path
if (-not (Test-Path "$repo\memory\.git")) {
  if ((Test-Path "$repo\memory") -and (Get-ChildItem "$repo\memory" -Force)) { throw "$repo\memory has files but isn't a clone of $($From): move them aside, then run this again" }
  git clone --branch $Branch --single-branch $From "$repo\memory"
  if ($LASTEXITCODE) { throw "Couldn't clone $($From)" }
}
# Claude Code names a project's folder after its path, with every other character turned into '-' (D:\cursora -> D--cursora).
$slug = $repo -replace '[^A-Za-z0-9]', '-'
$link = Join-Path $env:USERPROFILE ".claude\projects\$slug\memory"
if (Test-Path $link) {
  $item = Get-Item $link -Force
  if ($item.LinkType -eq 'Junction') { "Already linked: $link -> $($item.Target)"; exit }
  if (Get-ChildItem $link -Force) { throw "$link already has notes: copy any that memory\ lacks into it, then delete the folder and run this again" }
  Remove-Item $link
}
New-Item -ItemType Directory -Force (Split-Path $link) | Out-Null
New-Item -ItemType Junction -Path $link -Target "$repo\memory" | Out-Null
"Linked $link -> $repo\memory"
