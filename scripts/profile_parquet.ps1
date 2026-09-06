param(
    [int]$Parts = 10000,
    [int]$TestsPerPart = 5,
    [int]$BatchSize = 65536,
    [string]$Output = ""
)

$ErrorActionPreference = "Stop"

$args = @(
    "run", "-p", "stdf-parquet", "--bin", "parquet_perf", "--release", "--",
    "--parts", "$Parts",
    "--tests-per-part", "$TestsPerPart",
    "--batch-size", "$BatchSize"
)

if ($Output -ne "") {
    $args += @("--output", $Output)
}

cargo @args

Write-Host ""
Write-Host "Profiler workflow:"
Write-Host "1. For Visual Studio: open Performance Profiler and launch target/release/parquet_perf.exe with the same arguments."
Write-Host "2. For Windows Performance Recorder: record CPU usage while running this script, then inspect in Windows Performance Analyzer."
Write-Host "3. For WSL/Linux: use cargo flamegraph -p stdf-parquet --bin parquet_perf -- --parts $Parts --tests-per-part $TestsPerPart --batch-size $BatchSize."
