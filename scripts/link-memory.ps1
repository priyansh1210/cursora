# Points Claude Code's memory folder for this clone at the repo's memory/ folder, so every dev PC
# shares one memory through git. Run once per PC, from anywhere: powershell -File scripts\link-memory.ps1
# shortcut: Windows only; a Claude Code running inside WSL would need a symlink instead, if we ever run it there.
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path "$PSScriptRoot\..").Path
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
