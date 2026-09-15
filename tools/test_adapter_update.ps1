param([Parameter(Mandatory=$true)][string]$TestDirectory)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'Adapter-UpdatePlan.ps1')
$stage=Join-Path $TestDirectory 'stage'
$existing=Join-Path $TestDirectory 'existing'
foreach($arch in @('x86','x64')) {
 New-Item -ItemType Directory -Force -Path (Join-Path $stage $arch),(Join-Path $existing $arch) | Out-Null
 [IO.File]::WriteAllText((Join-Path $stage "$arch\a865r_bda.dll"),"new-$arch")
 Copy-Item -LiteralPath (Join-Path $stage "$arch\a865r_bda.dll") -Destination (Join-Path $existing "$arch\a865r_bda.dll")
}
$manifest=[pscustomobject]@{x86=(Get-FileHash (Join-Path $stage 'x86\a865r_bda.dll')).Hash;x64=(Get-FileHash (Join-Path $stage 'x64\a865r_bda.dll')).Hash}
$entries=@([pscustomobject]@{Arch='x86';Path=(Join-Path $existing 'x86\a865r_bda.dll');Machine=$true},[pscustomobject]@{Arch='x64';Path=(Join-Path $existing 'x64\a865r_bda.dll');Machine=$false})
if(@(Get-AdapterUpdatePlan @() $stage $manifest).Count -ne 0){throw 'No adapter should require no update'}
if(@(Get-AdapterUpdatePlan $entries $stage $manifest).Count -ne 0){throw 'Identical machine/user adapters must require no update or elevation'}
[IO.File]::WriteAllText($entries[0].Path,'old-x86')
$plan=@(Get-AdapterUpdatePlan ($entries+$entries[0]) $stage $manifest)
if($plan.Count -ne 1 -or -not $plan[0].Machine -or $plan[0].Destination -ne $entries[0].Path){throw 'Changed duplicate machine registration must produce one in-place update'}
Copy-Item -LiteralPath (Join-Path $stage 'x86\a865r_bda.dll') -Destination $entries[0].Path
[IO.File]::WriteAllText($entries[1].Path,'old-x64')
$plan=@(Get-AdapterUpdatePlan $entries $stage $manifest)
if($plan.Count -ne 1 -or $plan[0].Machine){throw 'Changed user adapter must preserve user scope'}
$missing=[pscustomobject]@{Arch='x86';Path=(Join-Path $existing 'missing\a865r_bda.dll');Machine=$true}
$plan=@(Get-AdapterUpdatePlan @($missing) $stage $manifest)
if($plan.Count -ne 1 -or $plan[0].Existed){throw 'Missing registered DLL must be offered for repair'}
[IO.File]::WriteAllText((Join-Path $stage 'x64\a865r_bda.dll'),'damaged-stage')
$rejected=$false
try {Get-AdapterUpdatePlan $entries $stage $manifest | Out-Null} catch {$rejected=$true}
if(-not $rejected){throw 'Damaged staged adapter must fail checksum validation'}
'PASS: absent/equal adapters, changed machine/user DLLs, duplicate registrations, and checksum rejection'
