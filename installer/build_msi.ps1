# Build "Nexulon Switcher Setup.msi" with the portable WiX 3.14 toolset.
#
#   powershell -ExecutionPolicy Bypass -File installer\build_msi.ps1
#
# WiX binaries are not committed: point -WixDir at an extracted
# wix314-binaries.zip (https://github.com/wixtoolset/wix3/releases).

param(
    [string]$WixDir = $env:WIX314,
    [string]$Output = ""
)

$ErrorActionPreference = "Stop"

$installer = $PSScriptRoot
$root = Split-Path -Parent $installer
if (-not $Output) {
    $Output = Join-Path $root "Nexulon Switcher Setup.msi"
}

if (-not $WixDir) {
    throw "Set -WixDir to the folder holding candle.exe and light.exe, or set the WIX314 environment variable."
}

$candle = Join-Path $WixDir "candle.exe"
$light = Join-Path $WixDir "light.exe"
if (-not (Test-Path $candle) -or -not (Test-Path $light)) {
    throw "WiX 3.14 binaries not found in $WixDir (need candle.exe and light.exe)."
}

$exe = Join-Path $root "target\release\nexulon-switcher.exe"
if (-not (Test-Path $exe)) {
    throw "Build the release exe first: cargo build --release"
}

# WebView2 is the app's only runtime dependency. The Evergreen bootstrapper is
# embedded in the MSI so setup can install it silently on machines that lack it.
$redist = Join-Path $installer "redist\MicrosoftEdgeWebView2Setup.exe"
if (-not (Test-Path $redist)) {
    throw "WebView2 bootstrapper missing: $redist`nDownload it from https://go.microsoft.com/fwlink/p/?LinkId=2124703"
}

$payload = Join-Path $installer "payload"
New-Item -ItemType Directory -Force -Path $payload | Out-Null
Copy-Item $exe (Join-Path $payload "Nexulon Switcher.exe") -Force

Push-Location $installer
try {
    # Releases keep the same version while their files are replaced, so every
    # build gets a fresh product code and the MSI performs a same-version major
    # upgrade. Without that, installing over the previous build would open a
    # repair that keeps the old exe because its file version did not change.
    $productCode = [guid]::NewGuid().ToString().ToUpper()
    $wxs = Get-Content "NexulonSwitcher.wxs" -Raw
    $wxs = [regex]::Replace($wxs, '<Product Id="[0-9A-Fa-f-]{36}"', "<Product Id=`"$productCode`"")
    [IO.File]::WriteAllText(
        (Join-Path $installer "NexulonSwitcher.build.wxs"),
        $wxs,
        (New-Object Text.UTF8Encoding($false))
    )

    & $candle -nologo -arch x64 -ext WixUIExtension "NexulonSwitcher.build.wxs"
    if ($LASTEXITCODE -ne 0) { throw "candle failed ($LASTEXITCODE)" }

    & $light -nologo -ext WixUIExtension -cultures:en-us -sice:ICE43 -sice:ICE57 -sice:ICE61 -out $Output "NexulonSwitcher.build.wixobj"
    if ($LASTEXITCODE -ne 0) { throw "light failed ($LASTEXITCODE)" }
} finally {
    Remove-Item "NexulonSwitcher.build.wxs", "NexulonSwitcher.build.wixobj", "NexulonSwitcher.build.wixpdb" -Force -ErrorAction SilentlyContinue
    Pop-Location
}

# `light` writes debug symbols next to the MSI; the deliverable does not need them.
$pdb = [IO.Path]::ChangeExtension($Output, ".wixpdb")
if (Test-Path $pdb) { Remove-Item $pdb -Force }

Get-Item $Output | ForEach-Object { "built {0} ({1:N0} bytes)" -f $_.FullName, $_.Length }
