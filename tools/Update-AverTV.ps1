param([Parameter(Mandatory=$true)][string]$InstallDirectory,[switch]$CheckOnly,[switch]$Elevated)
try {
 $ErrorActionPreference='Stop'
 $install=(Resolve-Path -LiteralPath $InstallDirectory).Path
 if(-not (Test-Path -LiteralPath (Join-Path $install 'installed-mode.txt'))){throw 'Not an Open Volar S installation.'}
 . (Join-Path $PSScriptRoot 'Adapter-UpdatePlan.ps1')
 $stage=Join-Path $install 'adapter-update'
 $manifest=Get-Content -LiteralPath (Join-Path $stage 'SHA256.json') -Raw | ConvertFrom-Json
 $registrations=@()
 foreach($arch in @('x86','x64')) {
  $view=if($arch -eq 'x86'){[Microsoft.Win32.RegistryView]::Registry32}else{[Microsoft.Win32.RegistryView]::Registry64}
  foreach($hive in @([Microsoft.Win32.RegistryHive]::CurrentUser,[Microsoft.Win32.RegistryHive]::LocalMachine)) {
   $root=[Microsoft.Win32.RegistryKey]::OpenBaseKey($hive,$view)
   try {foreach($clsid in @('{2A5FC455-33C1-482C-9A83-4DF863F0C601}','{2A5FC455-33C1-482C-9A83-4DF863F0C602}')) {
    $key=$root.OpenSubKey("Software\Classes\CLSID\$clsid\InprocServer32")
    try {if($key){$registrations+=[pscustomobject]@{Arch=$arch;Path=[string]$key.GetValue('');Machine=($hive -eq [Microsoft.Win32.RegistryHive]::LocalMachine)}}}
    finally {if($key){$key.Dispose()}}
   }} finally {$root.Dispose()}
  }
 }
 $plan=@(Get-AdapterUpdatePlan -Registrations $registrations -Stage $stage -Manifest $manifest)
 # Identical DLLs (or no adapter) need no action, even if AverTV is open.
 if($plan.Count -eq 0){exit 0}
 if($CheckOnly){exit 2}
 if(Get-Process -Name AVerTV -ErrorAction SilentlyContinue){throw 'Close AverTV before updating its adapter. Live TV can still be launched.'}
 $admin=([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
 $service=Get-Service -Name AVerRemote -ErrorAction SilentlyContinue
 $restart=$service -and $service.Status -eq 'Running'
 $protected=@($env:ProgramFiles,${env:ProgramFiles(x86)},$env:windir) | Where-Object {$_}
 $needsAdmin=$restart -or @($plan | Where-Object {$_.Machine}).Count -gt 0
 foreach($item in $plan){foreach($prefix in $protected){if($item.Destination.StartsWith($prefix.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)){$needsAdmin=$true}}}
 if($needsAdmin -and -not $admin) {
  if($Elevated){throw 'Administrator approval was not granted for the adapter update.'}
  $arguments='-NoProfile -ExecutionPolicy Bypass -File "'+$PSCommandPath+'" -InstallDirectory "'+$install+'" -Elevated'
  $child=Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -Verb RunAs -WindowStyle Hidden -ArgumentList $arguments -PassThru -Wait
  exit $child.ExitCode
 }
 # Preserve existing COM paths and update only DLLs whose content changed.
 $backup=Join-Path $install ('compatibility\backup-'+[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfff'))
 New-Item -ItemType Directory -Path $backup | Out-Null
 $changed=@();$paused=$false
 try {
  if($restart){Stop-Service AVerRemote;$paused=$true;(Get-Service AVerRemote).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(15))}
  foreach($item in $plan) {
   $copy=Join-Path $backup ($changed.Count.ToString()+'.dll')
   if($item.Existed){Copy-Item -LiteralPath $item.Destination -Destination $copy}
   $changed+=[pscustomobject]@{Destination=$item.Destination;Backup=$copy;Existed=$item.Existed}
   New-Item -ItemType Directory -Force -Path (Split-Path -Parent $item.Destination) | Out-Null
   Copy-Item -LiteralPath $item.Source -Destination $item.Destination -Force
   if((Get-FileHash -LiteralPath $item.Destination -Algorithm SHA256).Hash -ine $item.Hash){throw 'Installed adapter verification failed'}
  }
  @{Success=$true;Backup=$backup;Adapters=$plan} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $backup 'update-result.json') -Encoding UTF8
 } catch {
  foreach($item in $changed){if($item.Existed){Copy-Item -LiteralPath $item.Backup -Destination $item.Destination -Force}else{Remove-Item -LiteralPath $item.Destination -Force -ErrorAction SilentlyContinue}}
  throw
 } finally {if($paused){Start-Service AVerRemote}}
} catch {
 $message=$_.Exception.Message
 if(-not $CheckOnly){Add-Type -AssemblyName System.Windows.Forms;[System.Windows.Forms.MessageBox]::Show($message,'AverTV adapter update') | Out-Null}
 Write-Error $message -ErrorAction Continue
 exit 1
}
