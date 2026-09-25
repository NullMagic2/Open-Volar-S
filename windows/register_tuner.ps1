param([switch]$Unregister, [string]$Directory = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
# Register for the current user, with separate COM registry views for each bitness.
$system64 = if ([Environment]::Is64BitOperatingSystem -and -not [Environment]::Is64BitProcess) { 'Sysnative' } else { 'System32' }
foreach ($architecture in @('x64', 'x86')) {
    $dll = Join-Path $Directory "$architecture\a865r_bda.dll"
    if (-not (Test-Path -LiteralPath $dll)) { throw "Missing $architecture tuner adapter: $dll" }
    $system = if ($architecture -eq 'x86' -and [Environment]::Is64BitOperatingSystem) { 'SysWOW64' } else { $system64 }
    $program = Join-Path $env:SystemRoot "$system\regsvr32.exe"
    $arguments = @('/s', '/n', '/i:user')
    if ($Unregister) { $arguments += '/u' }
    $arguments += ('"' + $dll + '"')
    $process = Start-Process -FilePath $program -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Tuner registration failed for $architecture (exit $($process.ExitCode))." }
}
Write-Host $(if ($Unregister) { 'Open Volar S BDA tuner registration removed.' } else { 'Open Volar S BDA tuner registered. Restart VLC or your TV application.' })
