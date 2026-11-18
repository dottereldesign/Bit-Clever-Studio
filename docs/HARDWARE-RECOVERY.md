# Physical recovery qualification

Status: **not run on physical hardware**. Automated process termination, SQLite capacity exhaustion, corrupt temporary/published databases and renamed unavailable folders are covered by `full_recovery_tests`. None is equivalent to pulling a drive or cutting power.

Use a sacrificial Windows test machine and a spare drive containing only disposable fixtures. Do not run this on the user's PC, development library or only backup. Keep an immutable fixture and SHA-256 inventory on a separate unaffected machine/device. A VM forced shutdown is useful additional evidence, but does not qualify the physical controller or drive cache.

## Record the environment

Record the source commit, Windows version, drive/controller model and firmware, USB enclosure/bridge, filesystem, write-cache/removal policy, available space and fault method. Test each supported configuration separately (for example internal NTFS, USB NTFS and USB exFAT); a pass on one does not qualify another.

## Fault matrix

| Operation | Interruptions | Required observations |
| --- | --- | --- |
| Download and transcode | During transfer, transcode, publication and metadata save | No surviving worker after app death; recognised abandoned staging reclaimed on restart; Retry does not duplicate committed media |
| New library creation | Schema/import, checkpoint/publication, filing and activation | Unpublished recognised setup can retry; published corruption fails closed; previous selection stays intact until successful activation |
| Generated thumbnail replacement | Encoding, publication and metadata commit | Old image remains available on rollback; committed thumbnail decodes; Undo/Redo works |
| Import and file organisation | Copy, destination publication, source removal and commit | Originals or verified committed destinations survive; conflicting files are preserved; retries do not duplicate items |
| Library on removable drive | Disconnect while idle and during each write phase; reconnect | Clear unavailable/recovery error, no silent fallback or replacement; recovery succeeds or preserves evidence with an explicit error |

Repeat each boundary with process termination, actual spare-drive disconnect and controlled test-machine power loss. Randomise timing in repeated trials and record trial counts; finite successes do not prove all possible interruption timings. Do not use production credentials or third-party video services: serve deterministic media from a separate local fixture server.

## Verify every trial

1. Preserve a copy of the affected fixture and logs before repair/retry. Do not remove SQLite WAL/journal files or run filesystem repair first.
2. Reconnect and reopen normally. Record messages and selected-library pointer before and after recovery.
3. On the disposable copy, check `PRAGMA integrity_check`, foreign-key consistency, expected item/history counts and file references. Compare original and committed media hashes against the independent inventory; decode generated thumbnails.
4. Retry the operation, then check idempotency and relevant Undo/Redo. Confirm no download descendants survive and cleanup preserves unknown/conflicting files.
5. Acknowledged commits must survive when the tested storage configuration honours flushes. Unacknowledged operations may roll back or complete consistently. Missing acknowledged data, inconsistent references, silent overwrite, incorrect active selection or deletion of unknown data is a failure to investigate, not a pass masked by retry.

Store a report with configuration, fault phase, trial count, results and retained evidence. Leave untested rows marked **not run**. If device/OS flush guarantees fail, document the unsupported configuration rather than claiming the application can repair arbitrary hardware corruption.
