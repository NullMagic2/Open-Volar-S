# Pure planning: no registration changes, service operations, or elevation.
function Get-AdapterUpdatePlan {
 param([array]$Registrations,[string]$Stage,$Manifest)
 $items=@{}
 foreach($entry in $Registrations) {
  $source=Join-Path $Stage ($entry.Arch+'\a865r_bda.dll')
  $expected=$Manifest.($entry.Arch)
  if(-not $expected -or (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ine $expected){throw "Adapter checksum failed: $($entry.Arch)"}
  $destination=$entry.Path
  if(-not [IO.Path]::IsPathRooted($destination) -or [IO.Path]::GetFileName($destination) -ine 'a865r_bda.dll'){throw "Unrecognized adapter registration: $destination"}
  $existed=Test-Path -LiteralPath $destination -PathType Leaf
  if($existed -and (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -ieq $expected){continue}
  $key=[IO.Path]::GetFullPath($destination).ToLowerInvariant()
  if($items.ContainsKey($key)) {
   if($items[$key].Arch -ne $entry.Arch){throw 'Conflicting adapter architectures share one DLL path.'}
   $items[$key].Machine=$items[$key].Machine -or $entry.Machine
  } else {
   $items[$key]=[pscustomobject]@{Arch=$entry.Arch;Source=$source;Destination=$destination;Hash=$expected;Machine=[bool]$entry.Machine;Existed=$existed}
  }
 }
 @($items.Values)
}
