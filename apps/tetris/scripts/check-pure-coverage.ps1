[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$coverageTarget = Join-Path $workspace 'target\llvm-cov-target'

function Invoke-Coverage {
    param([string[]] $CoverageArguments)

    & cargo llvm-cov @CoverageArguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo llvm-cov failed with exit code $LASTEXITCODE"
    }
}

function Get-LatestCoverageObject {
    param(
        [string] $Directory,
        [string] $Filter
    )

    $object = Get-ChildItem -LiteralPath $Directory -Filter $Filter -File |
        Sort-Object LastWriteTimeUtc -Descending |
        Select-Object -First 1
    if ($null -eq $object) {
        throw "coverage object not found: $Directory\$Filter"
    }
    $object.FullName
}

$sysroot = (& rustc --print sysroot).Trim()
$hostLine = & rustc -vV | Where-Object { $_ -like 'host:*' } | Select-Object -First 1
if ($null -eq $hostLine) {
    throw 'rustc did not report a host triple'
}
$hostTriple = $hostLine.Substring('host:'.Length).Trim()
$llvmCov = Join-Path $sysroot "lib\rustlib\$hostTriple\bin\llvm-cov.exe"
if (-not (Test-Path -LiteralPath $llvmCov -PathType Leaf)) {
    throw "llvm-cov not found: $llvmCov"
}

function Assert-FileCoverage {
    param(
        [string[]] $Objects,
        [string] $PathSuffix,
        [string] $Label
    )

    $profile = Get-LatestCoverageObject $coverageTarget '*.profdata'
    $llvmArguments = @('export', '-format=text', "-instr-profile=$profile")
    foreach ($object in $Objects) {
        $llvmArguments += "-object=$object"
    }

    $json = (& $llvmCov @llvmArguments | Out-String)
    if ($LASTEXITCODE -ne 0) {
        throw "llvm-cov export failed for $Label with exit code $LASTEXITCODE"
    }
    $report = $json | ConvertFrom-Json
    $normalizedSuffix = $PathSuffix.Replace('/', '\')
    $file = @($report.data[0].files) | Where-Object {
        $_.filename.Replace('/', '\').EndsWith($normalizedSuffix)
    }
    if ($file.Count -ne 1) {
        throw "expected one coverage record for $PathSuffix, found $($file.Count)"
    }

    foreach ($metricName in @('regions', 'functions', 'lines')) {
        $metric = $file[0].summary.$metricName
        if ($metric.covered -ne $metric.count) {
            throw "$Label $metricName coverage is $($metric.covered)/$($metric.count), expected 100%"
        }
    }

    $summary = $file[0].summary
    Write-Host (
        '{0}: regions {1}/{2}, functions {3}/{4}, lines {5}/{6}' -f
        $Label,
        $summary.regions.covered,
        $summary.regions.count,
        $summary.functions.covered,
        $summary.functions.count,
        $summary.lines.covered,
        $summary.lines.count
    )
}

Push-Location $workspace
try {
    Invoke-Coverage @(
        '--all-targets',
        '--locked',
        '--fail-under-lines', '100',
        '--fail-under-functions', '100',
        '--fail-under-regions', '100'
    )

    $examples = Join-Path $coverageTarget 'debug\examples'
    $terminalExample = Get-LatestCoverageObject $examples 'terminal-*.exe'

    Assert-FileCoverage `
        @($terminalExample) `
        'examples\terminal\view.rs' `
        'Tetris terminal view'
}
finally {
    Pop-Location
}
