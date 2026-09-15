param(
    [string]$InstallDirectory = 'C:\Program Files\A865R',
    [string]$ResultPath = (Join-Path $PSScriptRoot '..\debug\exports\avertv-startup\repair-result.json')
)
$ErrorActionPreference = 'Stop'
$result = [ordered]@{ success = $false; backup = $null; adapters = @(); error = $null }
$serviceWasRunning = $false
$changed = @()
try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not ([Security.Principal.WindowsPrincipal]::new($identity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Administrator rights are required to update the installed compatibility DLLs.'
    }
    $install = (Resolve-Path -LiteralPath $InstallDirectory).Path
    if (-not (Test-Path -LiteralPath (Join-Path $install 'installed-mode.txt'))) {
        throw 'Expected existing A865R installation marker is missing.'
    }
    $source = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\compatibility')).Path
    $expected = @{
        x86 = '95C1141037393AE15384584C9951FCAA9E593D5E3456DB4AFC205340325DEE10'
        x64 = '9A6A905A37A40AF07C445E4E962E77BB0089482BC70CC778B2D4356E5FC9CB87'
    }
    $plan = foreach ($arch in @('x86', 'x64')) {
        $src = Join-Path $source "$arch\a865r_bda.dll"
        $dst = Join-Path $install "compatibility\$arch\a865r_bda.dll"
        if ((Get-FileHash -LiteralPath $src -Algorithm SHA256).Hash -ne $expected[$arch]) {
            throw "Unexpected source DLL hash: $arch"
        }
        if (-not (Test-Path -LiteralPath $dst)) { throw "Installed DLL is missing: $dst" }
        $view = if ($arch -eq 'x86') { [Microsoft.Win32.RegistryView]::Registry32 } else { [Microsoft.Win32.RegistryView]::Registry64 }
        $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine, $view)
        try {
            foreach ($clsid in @('{2A5FC455-33C1-482C-9A83-4DF863F0C601}', '{2A5FC455-33C1-482C-9A83-4DF863F0C602}')) {
                $key = $base.OpenSubKey("Software\Classes\CLSID\$clsid\InprocServer32")
                try {
                    if (-not $key -or $key.GetValue('') -ine $dst) { throw "Unexpected $arch registration for $clsid" }
                } finally { if ($key) { $key.Dispose() } }
            }
        } finally { $base.Dispose() }
        [pscustomobject]@{ arch = $arch; source = $src; destination = $dst; sha256 = $expected[$arch] }
    }
    $backup = Join-Path $install ('compatibility\backup-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfff'))
    New-Item -ItemType Directory -Path $backup | Out-Null
    $result.backup = $backup
    foreach ($item in $plan) {
        Copy-Item -LiteralPath $item.destination -Destination (Join-Path $backup ($item.arch + '.dll'))
    }
    # Only the known TV application and its existing remote service are paused.
    $tvPath = 'C:\Program Files (x86)\AVerMedia\AVerTV 3D\AVerTV.exe'
    foreach ($process in @(Get-Process -Name AVerTV -ErrorAction SilentlyContinue)) {
        if ($process.Path -ine $tvPath) { throw 'Unexpected AVerTV process path; refusing to stop it.' }
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit(10000) | Out-Null
    }
    $remote = Get-Service -Name AVerRemote -ErrorAction SilentlyContinue
    $serviceWasRunning = $remote -and $remote.Status -eq 'Running'
    if ($serviceWasRunning) {
        Stop-Service -Name AVerRemote
        (Get-Service AVerRemote).WaitForStatus('Stopped', [TimeSpan]::FromSeconds(15))
    }
    try {
        foreach ($item in $plan) {
            $changed += $item
            Copy-Item -LiteralPath $item.source -Destination $item.destination -Force
            if ((Get-FileHash -LiteralPath $item.destination -Algorithm SHA256).Hash -ne $item.sha256) {
                throw "Installed DLL verification failed: $($item.arch)"
            }
        }
        $result.adapters = @($plan | Select-Object arch, destination, sha256)
        $result.success = $true
    } catch {
        foreach ($item in $changed) {
            Copy-Item -LiteralPath (Join-Path $backup ($item.arch + '.dll')) -Destination $item.destination -Force
        }
        throw
    }
} catch {
    $result.error = $_.Exception.Message
} finally {
    if ($serviceWasRunning) {
        try {
            Start-Service -Name AVerRemote
            (Get-Service AVerRemote).WaitForStatus('Running', [TimeSpan]::FromSeconds(15))
        } catch {
            $result.success = $false
            $result.error = "Remote-service restart failed: $($_.Exception.Message)"
        }
    }
    $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $ResultPath -Encoding UTF8
}
if (-not $result.success) { exit 1 }
