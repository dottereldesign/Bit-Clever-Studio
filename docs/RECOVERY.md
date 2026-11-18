# Interrupted duplicate operations

Duplicate removal and restoration now record a file-move intent in SQLite before changing files. The intent is cleared in the same transaction that commits the item's archive/recovery state and merged references. This closes the gap where an abrupt exit could move a file but roll back the database, leaving it at an unexpected location.

On startup, reopening an existing library, rescanning, duplicate retry, managed-file maintenance or full export, pending moves are reconciled first:

- An uncommitted move returns to the previous database state.
- A committed move stays committed; its journal row is already gone.
- If publication left two copies, the destination is removed only after both match the recorded SHA-256 and the original is retained.
- If files changed, a location is occupied by different content, or a drive is unavailable, the files and intent are retained and recovery reports an error. It does not overwrite conflicting content.
- Pending paths are rebased when a managed library moves.

The mover publishes without overwriting a destination. Same-volume moves use a hard link followed by source removal; the fallback copies into a temporary file, verifies and syncs it, publishes it without overwriting, then removes the source. Normal copy errors keep the source. A forced exit before publication can leave an unreferenced temporary file; no automatic deletion of unidentified files is attempted.

A partial duplicate-removal batch can be retried with the same keeper and IDs. Already-committed removals for that keeper are skipped; the returned count reports newly removed items. A repeated restore succeeds without changing files if the item is already active and its file exists. Invalid or unavailable recovery requests still report an error.

## Imports, capture files and managed filing

Managed file organisation now records move intent before moving an owned file and clears that intent in the same transaction as its item path and checksum updates. Reopening/reconciliation restores an uncommitted move to the recorded location. Committed moves remain in place, and rerunning organisation completes a partially processed library.

Metadata imports into managed libraries and capture-file imports journal newly created media/thumbnail copies after verification and before publication. The original remains untouched. If metadata does not commit, recovery removes only provisional copies whose recorded hash still matches and which are not referenced by an item. A successful transaction clears its journal atomically with the records. Retrying uses the existing content-matching rules rather than creating duplicate media. Import journal paths follow library relocation.

A changed or newly referenced provisional copy, or an unavailable parent folder, stops automatic cleanup and preserves the remaining journal and files. Restore availability or resolve the conflict before retrying. Normal metadata/capture errors also attempt reconciliation immediately. A forced exit during copying, before a verified copy can be journalled, may still leave an unidentified temporary file; these files are not automatically deleted. This does not remove the original or commit partial metadata.

## Downloads and cancellation

Downloaded videos now use the provisional-copy journal while saving. Copy bytes are verified and synced before publication; metadata commit clears the journal atomically. Recovery removes an unchanged, unreferenced provisional copy, while a committed video remains available. Retrying the same URL reuses the saved video and applies requested project/tag assignments.

Cancellation is scoped to a unique attempt token, including retries, so a delayed request cannot cancel the next attempt. The native save path checks cancellation before publication and commit. Once the transaction commits, success wins over a late cancellation. Other save errors are not reported as successful cancellation.

After restart, an interrupted active job becomes an error with a Retry explanation instead of automatically starting network work. This also applies when cancellation was pending: the app cannot assume an unacknowledged cancellation succeeded. Explicit Retry checks for an already-saved video first. Waiting queued jobs retain their existing FIFO behaviour and library isolation.

On Windows, a supervisor joins a kill-on-close Job Object before starting the downloader. The app owns the job handle; abrupt app termination kills the supervisor, downloader and transcoder descendants. Cancellation and normal completion also terminate remaining descendants before joining output readers. No shell interprets download arguments. Other operating systems retain the existing process behaviour; whole-tree crash cleanup is Windows-specific.

New network staging folders have an ownership marker and an exclusive lifetime lock. Startup and the next download remove abandoned, recognised staging; live locked folders, unmarked legacy folders, unknown markers and linked/reparse-point contents are preserved. A crash before the ownership marker is written can still leave an unidentified folder. Cleanup errors are diagnosed at startup and retried before a new download.

Tests forcibly kill an isolated app-owner process while fake downloader/transcoder workers hold exclusive files. They verify both descendants terminate, abandoned staging is removed, and live/unknown staging is preserved. Save-boundary exits, retries, cancellation and attempt isolation are also covered. No public video service or real library is used; service authentication, network reliability and physical power loss are not established by these tests.

## Library creation and generated thumbnails

Library creation holds an exclusive setup lock and builds its database in a marked private workspace. A retry verifies the source backup and rebuilds recognised unpublished setup, including a torn temporary database. Unknown setup files and legacy `Creating.sqlite` files are preserved for review. Publication follows an integrity check, completed WAL checkpoint and database flush, and cannot overwrite an existing database. A durable pending flag resumes file organisation after publication. The active-library pointer changes only after the new library passes verification. A corrupt published database is preserved and rejected, never silently replaced.

Generated website/link thumbnails are flushed and journalled before publication. Metadata commit clears the intent atomically. Recovery removes only unchanged unreferenced provisional files; source images and old thumbnails needed by Undo/Redo remain intact. Tests interrupt generation and replacement before/after publication and commit, and check history round trips.

## If recovery needs attention

The library can open with a recovery warning if a pending location is unavailable. Reconnect the drive, then use **Rescan library** or retry the interrupted operation. A conflict warning means different content was found: preserve both locations and resolve the conflict before retrying. The pending move is not discarded. Recovery warnings are visible on Home and the other main views, and clear after a successful recovered snapshot arrives.

Full export and managed-file maintenance stop if pending reconciliation cannot safely complete. This avoids recording a new inventory or exporting a full backup while the database and files disagree. The existing active-library pointer remains unchanged if switching libraries fails. An unavailable selected library can be reopened after its folder returns.

## Development validation

```powershell
npm run test:native -- interruption_tests
npm run test:native -- full_recovery_tests
```

The normal tests create temporary libraries and launch isolated child **test processes**, ending those children with exit code 86 at checkpoints without unwinding. They never launch, attach to or terminate Design Dev, and never use the user's library. The ignored `crash_worker` is invoked only by these parent tests; it is not a standalone manual test. Crash/copy-injection environment hooks are compiled only into test binaries.

Coverage additionally includes managed-filing and metadata/capture-file exits before and after publication/commit, provisional thumbnail copies, failed metadata saves, conflict preservation, library relocation and safe retries. Coverage includes exits after intent persistence, destination publication, source removal and metadata commit; hard-link and forced-copy paths; deferred SQLite commit failures; idempotent partial-batch retries; moved libraries; conflicting files; and a simulated unavailable recovery directory. A Windows file-sharing lock tests failure of atomic active-library pointer replacement, followed by retry. Renaming a fixture folder simulates drive unavailability and reconnection. Native-to-frontend contracts and browser tests also check ordinary export/recovery and the recovery warning's appearance/clearance.

Database connections explicitly use WAL with `synchronous=FULL`. Tests assert this setting, force SQLite capacity exhaustion, terminate workers before/after WAL commit, simulate unavailable library folders and preserve a corrupt published database. These establish selected process-exit and failure behaviour, not hardware durability. SQLite still depends on correct filesystem and device flush behaviour: see [SQLite synchronous](https://sqlite.org/pragma.html#pragma_synchronous) and [hardware/OS corruption limits](https://www.sqlite.org/howtocorrupt.html). Process-tree guarantees follow [Windows Job Object semantics](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

Actual physical drive removal, power loss, controller caches and filesystem damage remain unqualified. See the [hardware qualification procedure](HARDWARE-RECOVERY.md). Unidentified pre-journal files and older-build orphans are preserved for review. Keep verified backups; recovery journals are not backups.
