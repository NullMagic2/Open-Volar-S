# Start Menu adapter repair

The failing Start Menu launch (AVerTV PID 36680) loaded
`C:\Program Files\A865R\compatibility\x86\a865r_bda.dll`, an older build.
The working launch loaded the current project's user-registered adapter.
The conflicting old system hashes were:

- x86: AE8E53AF09CB4C9D14FD95AC3370A34C6ECDA8484ED30FE3C09A4FAF5EF38FAC
- x64: F5784A293ACB88B6A8188716EE0B108E61B04FDDE1577BE678604CF79F322B03

On September 10, both installed adapters were backed up and updated with
`tools/repair-installed-bda.ps1`, preserving existing COM registration and WinUSB.
The verified installed hashes are:

- x86: 95C1141037393AE15384584C9951FCAA9E593D5E3456DB4AFC205340325DEE10
- x64: 9A6A905A37A40AF07C445E4E962E77BB0089482BC70CC778B2D4356E5FC9CB87

AVerRemote was stopped briefly and restarted successfully. The old DLLs are
backed up under `C:\Program Files\A865R\compatibility\backup-20260910T164707991`.
No firmware was uploaded and no Windows startup entry was added. The user
removed the original AVerMedia driver independently; WinUSB remains bound and OK.

Launching the existing Start Menu shortcut after repair (PID 9056) loaded the
current user adapter, locked RF22, reached measured quality 100%, and delivered
37,156,832 bytes with no queue drops before closing. That window closed before
visual verification. The user subsequently confirmed that launching from the
Windows Start Menu now shows live TV. The loaded module path of that final
user-launched process was not separately recorded.

This is a repair for the existing installation, not a general installer. It
requires administrator rights, verifies expected source hashes and both existing
COM registrations before changing files, keeps backups, rolls back replaced DLLs
on a copy/verification failure, and restores the remote service if it was running.
Future releases must keep user and system adapter versions aligned. The main
GUI installer still does not manage the separate BDA adapter registration.

Local evidence: `debug/exports/avertv-startup/repair-result.json`; the successful
post-repair launch logged to `%LOCALAPPDATA%\A865R\Compatibility\adapter.log`.
The previously delivered 0.7.0 handoff archive predates this repair.
