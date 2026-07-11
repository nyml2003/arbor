[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$RecordsPath = Join-Path $PSScriptRoot "records.json"
$Records = Get-Content -Raw -LiteralPath $RecordsPath | ConvertFrom-Json
$Utf8 = [Text.UTF8Encoding]::new($false)
$Failures = [Collections.Generic.List[string]]::new()
$MetadataByWorkspace = @{}

function Get-FileSha256([string] $Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-TextSha256([string] $Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return ($sha.ComputeHash($Utf8.GetBytes($Text)) | ForEach-Object {
            $_.ToString("x2")
        }) -join ""
    }
    finally {
        $sha.Dispose()
    }
}

function Get-RepoRelativePath([string] $Path) {
    $fullPath = (Resolve-Path -LiteralPath $Path).Path
    if ($fullPath.Equals($RepoRoot, [StringComparison]::OrdinalIgnoreCase)) {
        return "."
    }

    $prefix = $RepoRoot.TrimEnd("\") + "\"
    if (-not $fullPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside the repository: $Path -> $fullPath"
    }
    return $fullPath.Substring($prefix.Length).Replace("\", "/")
}

function Get-CanonicalRepoPath([string] $Path) {
    $fullPath = (Resolve-Path -LiteralPath $Path).Path
    $prefix = $RepoRoot.TrimEnd("\") + "\"
    if (-not $fullPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path resolves outside the repository: $Path -> $fullPath"
    }
    return $fullPath
}

function Get-OrdinalSortedStrings([string[]] $Values) {
    $copy = [string[]] @($Values)
    [Array]::Sort($copy, [StringComparer]::Ordinal)
    return $copy
}

function Get-UpstreamExportSha256([string[]] $CrateDirectories) {
    $files = [Collections.Generic.List[IO.FileInfo]]::new()
    foreach ($crateDirectory in $CrateDirectories) {
        $crateRoot = Get-CanonicalRepoPath (Join-Path $RepoRoot $crateDirectory)
        $files.Add((Get-Item -LiteralPath (Join-Path $crateRoot "Cargo.toml")))
        foreach ($subdirectory in @("src", "fixtures", "tests")) {
            $candidate = Join-Path $crateRoot $subdirectory
            if (Test-Path -LiteralPath $candidate) {
                foreach ($file in Get-ChildItem -LiteralPath $candidate -Recurse -File) {
                    $files.Add($file)
                }
            }
        }
    }

    $relativePaths = Get-OrdinalSortedStrings @($files | ForEach-Object { Get-RepoRelativePath $_.FullName })
    $text = [Text.StringBuilder]::new()
    foreach ($relativePath in $relativePaths) {
        $null = $text.Append($relativePath)
        $null = $text.Append([char] 0)
        $null = $text.Append((Get-FileSha256 (Join-Path $RepoRoot $relativePath)))
        $null = $text.Append("`n")
    }
    return Get-TextSha256 $text.ToString()
}

function Assert-Equal([string] $Label, [object] $Actual, [object] $Expected) {
    if ($Actual -cne $Expected) {
        $Failures.Add("$Label expected '$Expected', got '$Actual'")
    }
}

if (Test-Path -LiteralPath (Join-Path $RepoRoot "Cargo.toml")) {
    $Failures.Add("Repository-root Cargo.toml must not exist")
}

foreach ($workspaceProperty in $Records.workspaces.PSObject.Properties) {
    $workspaceName = $workspaceProperty.Name
    $baseline = $workspaceProperty.Value
    $manifestPath = Get-CanonicalRepoPath (Join-Path $RepoRoot $baseline.root_manifest)
    $manifestDirectory = Split-Path $manifestPath -Parent

    $metadataJson = & cargo metadata --locked --format-version 1 --manifest-path $manifestPath
    $metadataExitCode = $LASTEXITCODE
    Write-Output "metadata[$workspaceName] exit=$metadataExitCode manifest=$($baseline.root_manifest)"
    if ($metadataExitCode -ne 0) {
        $Failures.Add("cargo metadata failed for $workspaceName with exit code $metadataExitCode")
        continue
    }
    $metadata = ($metadataJson -join "`n") | ConvertFrom-Json
    $MetadataByWorkspace[$workspaceName] = $metadata

    Assert-Equal "$workspaceName workspace_root" (Get-RepoRelativePath $metadata.workspace_root) (Get-RepoRelativePath $manifestDirectory)
    Assert-Equal "$workspaceName root_manifest_sha256" (Get-FileSha256 $manifestPath) $baseline.root_manifest_sha256
    Assert-Equal "$workspaceName lockfile_sha256" (Get-FileSha256 (Join-Path $manifestDirectory "Cargo.lock")) $baseline.lockfile_sha256

    $memberPackages = @($metadata.packages | Where-Object { $metadata.workspace_members -contains $_.id })
    $memberDirectories = Get-OrdinalSortedStrings @($memberPackages | ForEach-Object {
        Get-RepoRelativePath (Split-Path $_.manifest_path -Parent)
    })
    $memberListText = ($memberDirectories | ForEach-Object { "$_`n" }) -join ""
    Assert-Equal "$workspaceName sorted_member_list_sha256" (Get-TextSha256 $memberListText) $baseline.sorted_member_list_sha256

    $actualManifestPaths = Get-OrdinalSortedStrings @($memberPackages | ForEach-Object { Get-RepoRelativePath $_.manifest_path })
    $expectedManifestPaths = Get-OrdinalSortedStrings @($baseline.member_manifest_sha256_by_path.PSObject.Properties.Name)
    Assert-Equal "$workspaceName member manifest paths" ($actualManifestPaths -join "|") ($expectedManifestPaths -join "|")
    foreach ($manifestProperty in $baseline.member_manifest_sha256_by_path.PSObject.Properties) {
        $memberManifest = Join-Path $RepoRoot $manifestProperty.Name
        Assert-Equal "$workspaceName member manifest $($manifestProperty.Name)" (Get-FileSha256 $memberManifest) $manifestProperty.Value
        if ((Get-Content -Raw -LiteralPath $memberManifest) -match "(?m)^\s*[^#\r\n]+\bpath\s*=") {
            $Failures.Add("Member manifest contains a direct path dependency: $($manifestProperty.Name)")
        }
    }

    Assert-Equal "$workspaceName approved_upstream_export_sha256" `
        (Get-UpstreamExportSha256 @($baseline.approved_upstream_crates)) `
        $baseline.approved_upstream_export_sha256
}

foreach ($pathRecord in $Records.canonical_path_dependencies) {
    if (-not $MetadataByWorkspace.ContainsKey($pathRecord.consumer)) {
        $Failures.Add("Missing metadata for path consumer $($pathRecord.consumer)")
        continue
    }
    $metadata = $MetadataByWorkspace[$pathRecord.consumer]
    $consumerBaseline = $Records.workspaces.PSObject.Properties[$pathRecord.consumer].Value
    $consumerRoot = Split-Path (Join-Path $RepoRoot $consumerBaseline.root_manifest) -Parent
    $declaredTarget = Get-CanonicalRepoPath (Join-Path $consumerRoot $pathRecord.cargo_path)
    $approvedTarget = Get-CanonicalRepoPath (Join-Path $RepoRoot $pathRecord.approved_repo_relative_target)
    Assert-Equal "$($pathRecord.consumer) $($pathRecord.dependency) declared path" $declaredTarget $approvedTarget

    $resolvedDependencies = @($metadata.packages.dependencies | Where-Object {
        $_.name -eq $pathRecord.dependency -and $null -ne $_.path
    })
    if ($resolvedDependencies.Count -eq 0) {
        $Failures.Add("No resolved member dependency for $($pathRecord.consumer) -> $($pathRecord.dependency)")
        continue
    }
    foreach ($dependency in $resolvedDependencies) {
        Assert-Equal "$($pathRecord.consumer) $($pathRecord.dependency) metadata path" `
            (Get-CanonicalRepoPath $dependency.path) $approvedTarget
    }
    Write-Output "path[$($pathRecord.consumer):$($pathRecord.dependency)] target=$($pathRecord.approved_repo_relative_target)"
}

$lockfiles = @($Records.workspaces.PSObject.Properties | ForEach-Object {
    $manifest = Join-Path $RepoRoot $_.Value.root_manifest
    Get-CanonicalRepoPath (Join-Path (Split-Path $manifest -Parent) "Cargo.lock")
})
$uniqueLockfiles = @($lockfiles | Sort-Object -Unique)
Assert-Equal "independent lockfile path count" $uniqueLockfiles.Count $lockfiles.Count
Write-Output "lockfiles independent=$($uniqueLockfiles.Count -eq 4) count=$($uniqueLockfiles.Count)"

$allowedPrefixes = @(
    "apps/punctum/",
    "packages/vsh/",
    "apps/gen3-game/",
    "apps/tui-chater/",
    "workspace/manage/punctum-vsh-program.md",
    "workspace/manage/punctum-vsh-s0/"
)
$statusLines = @(& git -C $RepoRoot status --short --untracked-files=all)
if ($LASTEXITCODE -ne 0) {
    $Failures.Add("git status failed with exit code $LASTEXITCODE")
}
foreach ($statusLine in $statusLines) {
    if ($statusLine.Length -lt 4) {
        continue
    }
    $changedPath = $statusLine.Substring(3).Replace("\", "/")
    if ($changedPath.Contains(" -> ")) {
        $changedPath = $changedPath.Split(" -> ")[-1]
    }
    if (-not ($allowedPrefixes | Where-Object { $changedPath.StartsWith($_, [StringComparison]::Ordinal) })) {
        $Failures.Add("Changed path is outside the S0 write scope: $changedPath")
    }
}
Write-Output "scope changed_files=$($statusLines.Count)"

if ($Failures.Count -gt 0) {
    foreach ($failure in $Failures) {
        Write-Error $failure
    }
    exit 1
}

Write-Output "S0 verification passed"
exit 0
