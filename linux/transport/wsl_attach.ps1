$ErrorActionPreference = 'Stop'
$usbipd = Join-Path $env:ProgramFiles 'usbipd-win\usbipd.exe'
if (-not (Test-Path -LiteralPath $usbipd)) {
    throw 'usbipd-win is not installed on Windows.'
}
$line = (& $usbipd list | Where-Object { $_ -match '(?i)\b07ca:b865\b' } | Select-Object -First 1)
if (-not $line) {
    throw 'A865R (07ca:b865) is not connected to Windows.'
}
if ($line -match '(?i)attached') {
    exit 0
}
if ($line -match '(?i)not shared') {
    # USBPcap requires a forced bind. This prompts for administrator access once;
    # usbipd keeps the share registered for later WSL launches.
    $process = Start-Process -FilePath $usbipd -ArgumentList 'bind --hardware-id 07ca:b865 --force' -Verb RunAs -Wait -PassThru -WindowStyle Hidden
    if ($null -eq $process -or $process.ExitCode -ne 0) {
        throw 'Administrator approval for usbipd bind failed.'
    }
}
& $usbipd attach --wsl --hardware-id 07ca:b865
if ($LASTEXITCODE -ne 0) {
    throw "usbipd attach failed with exit code $LASTEXITCODE."
}
