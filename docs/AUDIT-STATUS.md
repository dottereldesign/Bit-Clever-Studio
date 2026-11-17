# Audit follow-up status — 18 November 2026

This tracks the supplied audit across the three implementation batches. It is not a new security audit or a claim that every possible interaction has been tested.

## Completed

The first batch (`cb1abec`) addressed duplicate memberships/references, native millisecond timestamps in reporting, library-bound download queues, moved-library history/recovery paths, library-specific media caches, project-only link capture and sidebar visibility preferences.

The reliability batch (`a88fc45`) addressed errors reported after committed saves, undo activity affecting unrelated projects, and downloads holding the shared write lock during network/processing work. Derived-output failures now leave a repairable maintenance warning. Full filesystem journalling remains outside that change.

The testing/workflow batch repairs the 13 failing selected browser fixtures from the audit, adds an A→B→A→B queue/preferences contract and a frozen native-generated timestamp fixture, expands Windows CI browser coverage, and updates development/performance instructions. Browser checks exposed four additional interaction fixes: project rating buttons retain Enter activation; project file rows participate in pointer dragging; Quick preview restores focus in Strict Mode; and search shortcuts work with the nonmodal side viewer open.

The local checks use synthetic libraries and disposable browser profiles. They do not touch the real library, clipboard, external video services or calendar credentials. See [development checks](DEVELOPMENT.md#checks) and [native fixture provenance](../tests/fixtures/README.md).

## Still outstanding

1. **Dependency maintenance.** The audit's SQLite/rusqlite and Vite patch work, local Node alignment with CI, and Picomatch monitoring remain separate. Recheck current advisories and supported versions before changing lockfiles; validate database migration and backup compatibility. Other Rust notices still need monitoring for any future non-Windows targets.
2. **Refresh consistency.** Reproduce controlled out-of-order refresh responses, then address stale frontend results and consistent native read snapshots where needed.
3. **Preference recovery.** Validate parsed preference shapes (including valid JSON such as `null`) and provide a recoverable reset path. Parsing JSON alone is insufficient.
4. **Accessibility coverage.** Complete full-app Duplicates dialog and portalled picker focus checks, keyboard/screen-reader testing, contrast, themes and minimum-window coverage. The Quick preview focus fix does not resolve or verify every dialog.
5. **Representative performance profiling.** Measure 4,000/20,000-item startup, scrolling, cold/warm caches, rapid navigation/cancellation, native lock wait, database work, serialisation and React frame time. The repaired 1,200-item browser fixture is a synthetic development workload, not a production performance guarantee.
6. **Broader contracts and recovery acceptance.** Extend native-to-frontend contracts beyond the frozen won-lead fixture; strengthen combined duplicate removal/recovery/export/import, moved-library recovery, preference undo/restore and failure-repair scenarios. The browser suite still contains mocked backend logic and is not a substitute for native transaction tests.
7. **Maintainability.** Centralise reference remapping and timestamp/change contracts, extract refresh/history coordination from App, and reduce broad snapshot work where profiling or correctness evidence justifies it. These are targeted follow-ups, not a framework rewrite.
8. **Backup clarity and verification.** Add a user-facing coverage summary for exclusions such as My Business data, credentials, undo history and temporary queues, plus a disposable restore-verification command.

Standalone builds, installers, version bumps, tags and releases were explicitly excluded from these batches. A pushed development branch is a source backup; it does not update the standalone executable or imply integration into main.
