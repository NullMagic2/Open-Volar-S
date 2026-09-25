param([string]$Compiler = 'ISCC.exe', [string]$RuntimeDirectory = '')
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Push-Location -LiteralPath $projectDir
try {
    New-Item -ItemType Directory -Force -Path 'windows/bin', 'debug', 'GUI/Windows' | Out-Null
    cargo build --workspace --exclude open-volar-s-wsl-video-host --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    Copy-Item -LiteralPath 'target/release/live-tv.exe' -Destination 'GUI/Windows/live-tv.exe'
    Copy-Item -LiteralPath 'target/release/a865r-debug.exe' -Destination 'debug/a865r-debug.exe'
    Copy-Item -LiteralPath 'target/release/a865rctl.exe' -Destination 'windows/bin/a865rctl.exe'
    foreach ($dll in @('msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll')) {
        if ($RuntimeDirectory) {
            Copy-Item -LiteralPath (Join-Path $RuntimeDirectory $dll) -Destination (Join-Path 'GUI/Windows' $dll)
        }
        if (-not (Test-Path -LiteralPath (Join-Path 'GUI/Windows' $dll))) {
            throw 'Pass -RuntimeDirectory pointing to the x64 Microsoft.VC145.CRT folder from the Visual Studio redistributable.'
        }
    }
    cargo build -p a865r-bda --release --locked --target i686-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw '32-bit AverTV adapter build failed' }
    New-Item -ItemType Directory -Force -Path 'windows/adapter-update/x86','windows/adapter-update/x64' | Out-Null
    Copy-Item -LiteralPath 'target/i686-pc-windows-msvc/release/a865r_bda.dll' -Destination 'windows/adapter-update/x86/a865r_bda.dll'
    Copy-Item -LiteralPath 'target/release/a865r_bda.dll' -Destination 'windows/adapter-update/x64/a865r_bda.dll'
    @{x86=(Get-FileHash -LiteralPath 'windows/adapter-update/x86/a865r_bda.dll').Hash;x64=(Get-FileHash -LiteralPath 'windows/adapter-update/x64/a865r_bda.dll').Hash} | ConvertTo-Json | Set-Content -LiteralPath 'windows/adapter-update/SHA256.json' -Encoding UTF8
    & $Compiler (Join-Path $PSScriptRoot 'a865r.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
} finally { Pop-Location }
