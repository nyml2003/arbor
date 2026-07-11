[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$repo = Split-Path -Parent (Split-Path -Parent $workspace)
$targetRoot = Join-Path $repo '.target/tasks/f1/battle-local'
$manifest = Join-Path $workspace 'Cargo.toml'
$recordsPath = Join-Path $repo 'workspace/manage/punctum-vsh-s0/records.json'

function Invoke-Checked {
    param(
        [Parameter(Mandatory)]
        [string] $Name,

        [Parameter(Mandatory)]
        [scriptblock] $Command
    )

    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

function Assert-Coverage {
    param(
        [Parameter(Mandatory)]
        [string] $ReportPath,

        [Parameter(Mandatory)]
        [string] $SourcePattern
    )

    $report = Get-Content -Raw -LiteralPath $ReportPath | ConvertFrom-Json
    $files = @($report.data.files | Where-Object {
        $_.filename -like $SourcePattern -and $_.filename -notlike '*tests.rs'
    })
    if ($files.Count -eq 0) {
        throw "Coverage report did not contain files matching $SourcePattern"
    }

    foreach ($file in $files) {
        $metrics = @(
            @('regions', $file.summary.regions),
            @('functions', $file.summary.functions),
            @('lines', $file.summary.lines),
            @('branches', $file.summary.branches)
        )
        foreach ($metric in $metrics) {
            $name = $metric[0]
            $summary = $metric[1]
            if ($summary.count -gt 0 -and $summary.covered -ne $summary.count) {
                throw "$($file.filename) has $($summary.covered)/$($summary.count) covered $name"
            }
        }
    }
}

function Assert-BattleFixture {
    $records = Get-Content -Raw -LiteralPath $recordsPath | ConvertFrom-Json
    $gate = $records.gates.'BATTLE-RULES-v0.1'
    if ($gate.status -ne 'Approved') {
        throw 'BATTLE-RULES-v0.1 is not approved'
    }

    $fixturePath = Join-Path $repo $gate.canonical_fixture_bundle_path
    if (-not (Test-Path -LiteralPath $fixturePath -PathType Leaf)) {
        throw "Battle fixture does not exist: $fixturePath"
    }

    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fixturePath).Hash.ToLowerInvariant()
    if ($actualHash -ne $gate.canonical_fixture_bundle_sha256) {
        throw "Battle fixture hash mismatch: expected $($gate.canonical_fixture_bundle_sha256), got $actualHash"
    }
}

Assert-BattleFixture

$env:CARGO_TARGET_DIR = Join-Path $targetRoot 'stable'
Invoke-Checked 'cargo test' {
    cargo test -p battle-domain -p battle-application --all-targets --locked --manifest-path $manifest
}
Invoke-Checked 'cargo check' {
    cargo check -p battle-domain -p battle-application --all-targets --locked --manifest-path $manifest
}
Invoke-Checked 'cargo clippy' {
    cargo clippy -p battle-domain -p battle-application --all-targets --locked --manifest-path $manifest -- -D warnings
}
Invoke-Checked 'cargo fmt' {
    cargo fmt -p battle-domain -p battle-application --manifest-path $manifest -- --check
}

$env:CARGO_TARGET_DIR = Join-Path $targetRoot 'domain-coverage'
$domainReport = Join-Path $targetRoot 'domain-coverage.json'
Invoke-Checked 'battle-domain coverage' {
    cargo +nightly llvm-cov --manifest-path $manifest -p battle-domain --lib --branch `
        --json --output-path $domainReport
}
Assert-Coverage -ReportPath $domainReport -SourcePattern '*battle-domain\src\*.rs'

$env:CARGO_TARGET_DIR = Join-Path $targetRoot 'application-coverage'
$applicationReport = Join-Path $targetRoot 'application-coverage.json'
Invoke-Checked 'battle-application coverage' {
    cargo +nightly llvm-cov --manifest-path $manifest -p battle-application --lib --branch `
        --ignore-filename-regex 'battle-domain' `
        --json --output-path $applicationReport
}
Assert-Coverage -ReportPath $applicationReport -SourcePattern '*battle-application\src\*.rs'
