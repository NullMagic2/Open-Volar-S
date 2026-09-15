$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build -p a865r-debug --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    Copy-Item -LiteralPath 'target\release\a865r-debug.exe' -Destination (Join-Path $PSScriptRoot 'a865r-debug.exe') -Force
    Write-Host 'Built debug\a865r-debug.exe'
} finally {
    Pop-Location
}
