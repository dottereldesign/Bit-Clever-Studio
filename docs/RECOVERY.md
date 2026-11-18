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

Tests cover publication/commit process exits, safe retries and assignments, cancellation at save boundaries, late-token isolation, and stopping an isolated running child with partial staging cleanup. They do not use public video services. Abrupt termination of the entire app/downloader process tree during network transfer, orphaned staging cleanup after such termination, real-service authentication/retries, and power loss remain separate acceptance work.

## If recovery needs attention

The library can open with a recovery warning if a pending location is unavailable. Reconnect the drive, then use **Rescan library** or retry the interrupted operation. A conflict warning means different content was found: preserve both locations and resolve the conflict before retrying. The pending move is not discarded. Recovery warnings are visible on Home and the other main views, and clear after a successful recovered snapshot arrives.

Full export and managed-file maintenance stop if pending reconciliation cannot safely complete. This avoids recording a new inventory or exporting a full backup while the database and files disagree. The existing active-library pointer remains unchanged if switching libraries fails. An unavailable selected library can be reopened after its folder returns.

## Development validation

```powershell
npm run test:native -- interruption_tests
```

The normal tests create temporary libraries and launch isolated child **test processes**, ending those children with exit code 86 at checkpoints without unwinding. They never launch, attach to or terminate Design Dev, and never use the user's library. The ignored `crash_worker` is invoked only by these parent tests; it is not a standalone manual test. Crash/copy-injection environment hooks are compiled only into test binaries.

Coverage additionally includes managed-filing and metadata/capture-file exits before and after publication/commit, provisional thumbnail copies, failed metadata saves, conflict preservation, library relocation and safe retries. Coverage includes exits after intent persistence, destination publication, source removal and metadata commit; hard-link and forced-copy paths; deferred SQLite commit failures; idempotent partial-batch retries; moved libraries; conflicting files; and a simulated unavailable recovery directory. A Windows file-sharing lock tests failure of atomic active-library pointer replacement, followed by retry. Renaming a fixture folder simulates drive unavailability and reconnection. Native-to-frontend contracts and browser tests also check ordinary export/recovery and the recovery warning's appearance/clearance.

These tests establish selected process-exit and failure behaviour, not hardware power-loss durability. They do not emulate controller caches, filesystem damage, real removable-drive disconnection, every capture variant (including generated link/website thumbnails), download interruption, or crashes during all library-creation stages. Recovery of orphan files created by older builds without a journal is also outside this change. Keep verified backups; this journal is not a backup.
