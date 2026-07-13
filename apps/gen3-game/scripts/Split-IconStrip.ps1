[CmdletBinding()]
param(
    [Parameter(Mandatory, Position = 0)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$Source,

    [string]$OutputDirectory,

    [ValidateRange(1, 65535)]
    [int]$IconWidth = 32,

    [ValidateRange(1, 65535)]
    [int]$IconHeight = 16,

    [string]$Prefix = "icon",

    [switch]$Overwrite
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$sourcePath = (Resolve-Path -LiteralPath $Source).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $sourceItem = Get-Item -LiteralPath $sourcePath
    $OutputDirectory = Join-Path $sourceItem.DirectoryName "$($sourceItem.BaseName)-icons"
}
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)

$sourceImage = [System.Drawing.Bitmap]::new($sourcePath)
try {
    if ($sourceImage.Width % $IconWidth -ne 0 -or $sourceImage.Height % $IconHeight -ne 0) {
        throw "Image size $($sourceImage.Width)x$($sourceImage.Height) is not divisible by icon size ${IconWidth}x${IconHeight}."
    }

    $columnCount = [int]($sourceImage.Width / $IconWidth)
    $rowCount = [int]($sourceImage.Height / $IconHeight)
    $iconCount = $columnCount * $rowCount
    $digits = [Math]::Max(2, ($iconCount - 1).ToString().Length)
    $destinations = 0..($iconCount - 1) | ForEach-Object {
        Join-Path $outputPath ("{0}-{1}.png" -f $Prefix, $_.ToString("D$digits"))
    }

    $existing = @($destinations | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
    if ($existing.Count -gt 0 -and -not $Overwrite) {
        throw "$($existing.Count) output file(s) already exist. Pass -Overwrite to replace them."
    }

    New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
    for ($row = 0; $row -lt $rowCount; $row++) {
        for ($column = 0; $column -lt $columnCount; $column++) {
            $index = $row * $columnCount + $column
            $bounds = [System.Drawing.Rectangle]::new(
                $column * $IconWidth,
                $row * $IconHeight,
                $IconWidth,
                $IconHeight
            )
            $icon = $sourceImage.Clone($bounds, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
            try {
                $icon.Save($destinations[$index], [System.Drawing.Imaging.ImageFormat]::Png)
            }
            finally {
                $icon.Dispose()
            }
        }
    }

    Write-Host "Wrote $iconCount icons (${IconWidth}x${IconHeight}) to $outputPath"
}
finally {
    $sourceImage.Dispose()
}
