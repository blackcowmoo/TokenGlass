$ErrorActionPreference = "Stop"

if (-not [Environment]::Is64BitOperatingSystem -or [Environment]::Is64BitProcess -eq $false) {
  throw "Windows x64 환경에서 실행해야 합니다."
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$releaseDir = Join-Path $root "src-tauri\target\release"
$appPath = Join-Path $releaseDir "tokenglass.exe"
$sidecarPath = Join-Path $releaseDir "codex-x86_64-pc-windows-msvc.exe"

if (-not (Test-Path $appPath)) { throw "테스트 실행물이 없습니다. 먼저 pnpm build:windows-test를 실행하세요." }
if (-not (Test-Path $sidecarPath)) { throw "Windows x64 Codex sidecar가 테스트 실행물에 없습니다." }

$process = Start-Process -FilePath $appPath -PassThru
Start-Sleep -Seconds 3
if ($process.HasExited) { throw "TokenGlass가 시작 직후 종료되었습니다. 종료 코드: $($process.ExitCode)" }
Stop-Process -Id $process.Id -Force
Write-Host "Windows x64 smoke test passed: app and sidecar are present, and the app started."
