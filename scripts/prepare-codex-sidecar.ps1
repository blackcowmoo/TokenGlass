param(
  [Parameter(Mandatory = $true)]
  [string]$BinariesDir
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $BinariesDir | Out-Null
$env:CODEX_INSTALL_DIR = $BinariesDir
$env:CODEX_NON_INTERACTIVE = "1"
Invoke-RestMethod https://chatgpt.com/codex/install.ps1 | Invoke-Expression

if (-not (Test-Path (Join-Path $BinariesDir "codex.exe"))) {
  throw "The Codex installer did not produce codex.exe."
}

Move-Item -Force (Join-Path $BinariesDir "codex.exe") (Join-Path $BinariesDir "codex-x86_64-pc-windows-msvc.exe")
