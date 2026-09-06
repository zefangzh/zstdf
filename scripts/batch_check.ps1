<#
.SYNOPSIS
  Batch-check a folder tree of STDF files for structural/format sanity and
  produce one consolidated text report.

.DESCRIPTION
  Wraps the Rust `batch_check` binary. Builds a release binary once (fast
  on subsequent runs — cargo skips the rebuild if nothing changed) and then
  scans the given root directory recursively for *.stdf files, checking
  each one in-process with multiple worker threads (no per-file process
  spawn overhead — important when scanning TBs of data across thousands
  of files).

.PARAMETER RootDir
  Directory to scan recursively for *.stdf / *.std files.

.PARAMETER ReportPath
  Where to write the consolidated text report. Defaults to
  "stdf_batch_report_<timestamp>.txt" in the current directory.

.PARAMETER Threads
  Number of worker threads. Defaults to the number of logical CPUs
  (capped at 16). Lower this if the data lives on a slow network share
  and you are I/O bound rather than CPU bound.

.EXAMPLE
  .\batch_check.ps1 -RootDir "D:\fab_data\lot123"

.EXAMPLE
  .\batch_check.ps1 -RootDir "\\server\share\stdf_archive" -ReportPath "C:\reports\archive_report.txt" -Threads 8
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$RootDir,

    [string]$ReportPath = "stdf_batch_report_$(Get-Date -Format 'yyyyMMdd_HHmmss').txt",

    [int]$Threads = 0
)

$ErrorActionPreference = "Stop"

# Ensure cargo/rustc are on PATH for this session
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

# Resolve ReportPath to an absolute path *before* changing directories, so a
# relative path is anchored to the caller's original working directory
# rather than the Rust project directory we Push-Location into below.
if (-not [System.IO.Path]::IsPathRooted($ReportPath)) {
    $ReportPath = Join-Path (Get-Location).Path $ReportPath
}

$projectDir = Join-Path $PSScriptRoot ".." | Resolve-Path

# Safe default in case the try block terminates abnormally before reaching
# the real assignment below (e.g. an unhandled error partway through) —
# guarantees `exit $exitCode` always references a defined, meaningful value
# (1 = failure) instead of an unset variable.
$exitCode = 1

Push-Location $projectDir
try {
    Write-Host "Building release binary (skips if already up to date)..." -ForegroundColor Cyan
    cargo build --release --bin batch_check --quiet
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    # Default thread count is capped at 16 (see docs above); an explicit
    # -Threads value from the caller is passed through uncapped.
    $threadArg = if ($Threads -gt 0) { $Threads } else { [Math]::Min([Environment]::ProcessorCount, 16) }

    Write-Host "Running batch check on: $RootDir" -ForegroundColor Cyan
    cargo run --release --bin batch_check -- "$RootDir" "$ReportPath" $threadArg
    $exitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}

if (Test-Path $ReportPath) {
    Write-Host "`nReport saved to: $(Resolve-Path $ReportPath)" -ForegroundColor Green
}

exit $exitCode
