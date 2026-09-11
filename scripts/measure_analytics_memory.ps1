param(
    [long[]]$Rows = @(100000, 1000000),
    [long]$MaxPeakMiB = 64,
    [long]$MaxGrowthMiB = 16
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $root
try {
    if ($Rows.Count -lt 1 -or ($Rows | Where-Object { $_ -lt 0 }) -or $MaxPeakMiB -le 0 -or $MaxGrowthMiB -lt 0) {
        throw 'Use nonnegative row counts and positive peak RSS allowance.'
    }
    cargo build -p stdf-analytics --bin spill_stress --offline
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    $exe = (Resolve-Path 'target/debug/spill_stress.exe').Path
    $baseline = $null
    foreach ($count in $Rows) {
        $id = [guid]::NewGuid().ToString('N')
        $stdout = Join-Path $root "target/analytics-$id.stdout.log"
        $stderr = Join-Path $root "target/analytics-$id.stderr.log"
        $process = Start-Process -FilePath $exe -ArgumentList "$count" -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        try {
            $peak = 0L
            while (-not $process.WaitForExit(50)) {
                $process.Refresh()
                $peak = [Math]::Max($peak, $process.PeakWorkingSet64)
            }
            Get-Content $stdout
            if ($process.ExitCode -ne 0) {
                Get-Content $stderr
                throw "Spill stress failed with exit $($process.ExitCode)"
            }
            if ($peak -eq 0) { throw 'Process exited before RSS could be sampled; use more rows.' }
            if ($null -eq $baseline) { $baseline = $peak }
            "rows=$count sampled_peak_rss_mib=$([Math]::Round($peak / 1MB, 2))"
            if ($peak -gt $MaxPeakMiB * 1MB -or $peak -gt $baseline + $MaxGrowthMiB * 1MB) {
                throw 'Spill RSS regression allowance exceeded'
            }
        } finally {
            $process.Dispose()
        }
    }
} finally {
    Pop-Location
}
