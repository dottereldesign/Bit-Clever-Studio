# Library backup coverage and restore verification

Use **Storage & Backup → Back Up Now** to export a portable library while the app is running. Copy the **entire exported folder** to a separate drive/location. Keep another copy before attempting a real restore. An `INCOMPLETE.txt` marker or unavailable-original warning means the export is not a complete backup.

## Coverage

| Preserved by a full library export | Not preserved or not restored |
| --- | --- |
| Available original media, saved URLs and link thumbnails | Missing originals, removed/archived items and duplicate-recovery files |
| Projects, nested folders, memberships, tags, favourites and ratings | Undo/Redo history (labels may appear in metadata, but the undo stack is not restored) |
| Project overviews/moodboards, library clients/leads, client records and activity | **My Business**, which uses a separate device-private `my-business.sqlite` database |
| Saved library appearance preferences, Home tiles and app-owned appearance images | Account/calendar credentials, unfinished drafts, download queues, browser sessions and caches |

My Business is distinct from the library's clients/leads. Library export does not back up that device-private database. There is no dedicated My Business export in this batch; preserve its database separately as part of a closed-app device/app-data backup. Do not assume copying a managed library folder includes device-private data.

**Open Existing Library** restores a backup as a separate managed library and switches to it. **Merge Backup** adds its records to the current library; it is not a rollback of the current library. Restoring exported appearance preferences is separate from reconstructing Undo/Redo history. A raw copy of a live managed library can contain more state than an export; this table describes **Back Up Now**, not every file in a raw folder copy.

## File verification versus a restore rehearsal

The app's **Verify Backup** checks copied files against the recorded sizes and SHA-256 checksums (or the live library inventory for a managed library). It does not import the backup into a new database. Its success alone does not prove that every restore operation will succeed.

The development command below performs an additional disposable restore. It requires the repository's Node/Rust/Windows development prerequisites; it does not build an installer, launch Design or switch the active library.

```powershell
# Synthetic fixtures only; no personal library or backup is read.
npm run verify:restore

# Explicitly selected, complete exported backup; the source remains unchanged.
npm run verify:restore -- --folder "D:\Backups\Bit Clever Backup - example"
```

The selected-folder mode:

1. Rejects live library folders, incomplete exports and backups listing unavailable originals.
2. Verifies source checksums, copies manifest-listed files to a temporary directory, then verifies the copy again.
3. Opens **only the temporary copy** as a managed library, importing metadata and restoring preferences using the native app code.
4. Checks the restored file inventory, SQLite integrity and foreign keys, domain record counts, and item curation (including project/tag/favourite names, design/component memberships, rating and project-only state).
5. Closes and removes the temporary restore before reporting success. It never calls library activation or writes the active-library pointer.

A successful run prints `RESTORE_VERIFICATION` followed by aggregate JSON and exits zero. Failure exits nonzero; errors may identify a failing backup file. No verification report is uploaded. The command needs temporary space for the copied backup and its restored database. If the process is forcibly terminated, its temporary directory may remain; it is never the active library.

This is native restore verification, not a visual acceptance test or an exhaustive semantic comparison of all nested records/preferences. The live contract tests cover selected remapped references and frontend consumers separately (`npm run test:contracts`). A backup currently in use or being modified should first be exported again into a fresh folder. No real user backup was used to validate this batch.
