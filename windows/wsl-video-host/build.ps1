param([Parameter(Mandatory=$true)][string]$FfmpegSdk)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$sdk = (Resolve-Path $FfmpegSdk).Path
if (!(Test-Path (Join-Path $sdk 'include/libavcodec/avcodec.h'))) {
    throw 'FFmpeg SDK must contain include, lib, and bin directories.'
}
$oldSdk = $env:OVS_FFMPEG_SDK
Push-Location $root
try {
    $env:OVS_FFMPEG_SDK = $sdk
    cargo build --release --locked -p open-volar-s-wsl-video-host
    if ($LASTEXITCODE -ne 0) { throw 'WSL host build failed.' }
    $bundle = Join-Path $root 'dist/wsl-video-host'
    New-Item -ItemType Directory -Force $bundle | Out-Null
    Copy-Item -LiteralPath 'target/release/open-volar-s-wsl-video-host.exe' -Destination $bundle
    foreach ($name in @('avformat-63.dll','avcodec-63.dll','avutil-61.dll','swresample-7.dll')) {
        Copy-Item -LiteralPath (Join-Path $sdk "bin/$name") -Destination $bundle
    }
    Copy-Item -LiteralPath (Join-Path $sdk 'LICENSE') -Destination (Join-Path $bundle 'FFMPEG-LICENSE.txt')
    Copy-Item -LiteralPath (Join-Path $sdk 'README.txt') -Destination (Join-Path $bundle 'FFMPEG-README.txt')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'README.md') -Destination $bundle
    Write-Output "Built WSL host bundle: $bundle"
} finally {
    $env:OVS_FFMPEG_SDK = $oldSdk
    Pop-Location
}
