param([switch]$Sleep, [ValidateRange(20,60)][int]$WakeSeconds = 40,
      [Parameter(Mandatory=$true)][string]$OutputPath)
$ErrorActionPreference = 'Stop'
# Explicit diagnostic only: no scheduled task or persistent power-policy changes.
# Win32 contracts: SetSuspendState and SetWaitableTimer, Microsoft Learn.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.ComponentModel;
public static class A865rSleepTest {
 [StructLayout(LayoutKind.Sequential)] struct Luid { public uint Low; public int High; }
 [StructLayout(LayoutKind.Sequential)] struct Privileges { public uint Count; public Luid Id; public uint Attributes; }
 [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool CloseHandle(IntPtr h);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr CreateWaitableTimer(IntPtr security,bool manual,string name);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetWaitableTimer(IntPtr timer,ref long due,int period,IntPtr callback,IntPtr arg,bool resume);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool CancelWaitableTimer(IntPtr timer);
 [DllImport("kernel32.dll")] static extern void SetLastError(uint error);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool OpenProcessToken(IntPtr process,uint access,out IntPtr token);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool LookupPrivilegeValue(string system,string name,out Luid luid);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool AdjustTokenPrivileges(IntPtr token,bool disable,ref Privileges desired,uint size,out Privileges previous,out uint returned);
 [DllImport("powrprof.dll",SetLastError=true)] [return:MarshalAs(UnmanagedType.U1)]
 static extern bool SetSuspendState([MarshalAs(UnmanagedType.U1)]bool hibernate,[MarshalAs(UnmanagedType.U1)]bool force,[MarshalAs(UnmanagedType.U1)]bool disableWake);
 public static string Run(bool sleep,int seconds,string path) {
  IntPtr token=IntPtr.Zero,timer=IntPtr.Zero; Privileges old=new Privileges(); bool adjusted=false;
  try {
   if(!OpenProcessToken(GetCurrentProcess(),0x28,out token)) throw new Win32Exception();
   Luid id;if(!LookupPrivilegeValue(null,"SeShutdownPrivilege",out id)) throw new Win32Exception();
   Privileges p=new Privileges{Count=1,Id=id,Attributes=2};uint returned;
   SetLastError(0);
   if(!AdjustTokenPrivileges(token,false,ref p,(uint)Marshal.SizeOf(typeof(Privileges)),out old,out returned)) throw new Win32Exception();
   int error=Marshal.GetLastWin32Error();if(error!=0) throw new Win32Exception(error);
   adjusted=true;
   timer=CreateWaitableTimer(IntPtr.Zero,true,null);if(timer==IntPtr.Zero) throw new Win32Exception();
   DateTime wake=DateTime.UtcNow.AddSeconds(seconds);long due=wake.ToFileTimeUtc();
   SetLastError(0);
   if(!SetWaitableTimer(timer,ref due,0,IntPtr.Zero,IntPtr.Zero,true)) throw new Win32Exception();
   error=Marshal.GetLastWin32Error();if(error!=0) throw new Win32Exception(error);
   string prefix="{\"wake_timer_armed\":true,\"shutdown_privilege_available\":true,\"requested_mode\":\"S3\",\"wake_utc\":\""+wake.ToString("o")+"\"";
   System.IO.File.WriteAllText(path,prefix+",\"sleep_requested\":"+(sleep?"true":"false")+",\"phase\":\"prepared\"}");
   if(!sleep) return "Wake timer and process privilege verified; no sleep requested.";
   DateTime before=DateTime.UtcNow;
   if(!SetSuspendState(false,false,false)) throw new Win32Exception();
   DateTime after=DateTime.UtcNow;
   System.IO.File.WriteAllText(path,prefix+",\"sleep_requested\":true,\"phase\":\"returned\",\"before_utc\":\""+before.ToString("o")+"\",\"after_utc\":\""+after.ToString("o")+"\",\"elapsed_seconds\":"+(after-before).TotalSeconds.ToString(System.Globalization.CultureInfo.InvariantCulture)+"}");
   return "Suspend call returned. Verify Windows event log and tuner recovery separately.";
  } finally {
   if(timer!=IntPtr.Zero){CancelWaitableTimer(timer);CloseHandle(timer);}
   if(adjusted){Privileges ignored;uint length;AdjustTokenPrivileges(token,false,ref old,(uint)Marshal.SizeOf(typeof(Privileges)),out ignored,out length);}
   if(token!=IntPtr.Zero)CloseHandle(token);
  }
 }
}
'@
$sleepOutput = [System.IO.Path]::GetFullPath($OutputPath)
[A865rSleepTest]::Run($Sleep.IsPresent,$WakeSeconds,$sleepOutput)
