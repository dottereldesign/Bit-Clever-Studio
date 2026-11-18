# Audit follow-up status — 18 November 2026

This tracks the supplied audit across the implementation batches. It is not a new security audit or a claim that every possible interaction has been tested.

## Completed

The first batch (`cb1abec`) addressed duplicate memberships/references, native millisecond timestamps in reporting, library-bound download queues, moved-library history/recovery paths, library-specific media caches, project-only link capture and sidebar visibility preferences.

The reliability batch (`a88fc45`) addressed errors reported after committed saves, undo activity affecting unrelated projects, and downloads holding the shared write lock during network/processing work. Derived-output failures now leave a repairable maintenance warning. Full filesystem journalling remains outside that change.

The testing/workflow batch repairs the 13 failing selected browser fixtures from the audit, adds an A→B→A→B queue/preferences contract and a frozen native-generated timestamp fixture, expands Windows CI browser coverage, and updates development/performance instructions. Browser checks exposed four additional interaction fixes: project rating buttons retain Enter activation; project file rows participate in pointer dragging; Quick preview restores focus in Strict Mode; and search shortcuts work with the nonmodal side viewer open.

The local checks use synthetic libraries and disposable browser profiles. They do not touch the real library, clipboard, external video services or calendar credentials. See [development checks](DEVELOPMENT.md#checks) and [native fixture provenance](../tests/fixtures/README.md).

## Dependency maintenance — 18 November 2026

The dependency batch updates Vite 6.4.4 → 6.4.6 and rusqlite 0.37.0 → 0.41.1, with libsqlite3-sys 0.39.1 bundling SQLite 3.54.1. These address the [Vite file-deny bypass](https://github.com/advisories/GHSA-8g5g-mxqh-grgj) and [bundled SQLite WAL memory-exposure advisory](https://rustsec.org/advisories/RUSTSEC-2026-0459.html). Node 22.23.5 is pinned in `.node-version` for CI and the Windows development launcher; a checksum-verified, project-local setup avoids changing system Node. Rust 1.89 or newer is required by the updated dependency. The new rusqlite `fallible_uint` feature preserves the existing checked unsigned-integer conversions, and the two SQLite backup calls explicitly type their absent progress callback for the updated API.

The npm audit still reports four affected dependency nodes, all stemming from [Picomatch's repeated-extglob advisory](https://github.com/advisories/GHSA-fc5q-39xc-vjpf). Picomatch 4.0.7 remains the latest published version and the advisory lists no patch. No forced downgrade, advisory suppression or custom dependency patch was applied. This is build/development tooling; the dev server remains loopback-only.

An OSV scan of all 543 locked Rust dependencies no longer flags libsqlite3-sys. Seven other flagged packages (chrono, glib, proc-macro-error, rustix, zbus_names, zvariant and zvariant_derive) are absent from the Windows x64 dependency tree. Their advisories still need review before supporting other targets. These checks are a dated dependency inventory, not a claim of zero vulnerabilities.

Validation for this batch: clean npm install, 101 frontend tests, 103 native tests (four existing ignored tests), all ten browser smoke workflows at both configured viewport sizes, TypeScript/frontend build, version consistency, diagnostics guard, launcher prerequisite check and staged secret scan passed. Native coverage includes migrations, SQLite-backed export/restore and the earlier correctness/reliability regressions. The user confirmed Design Dev opens with the expected library/projects and a rating change saves and can be undone.

## Still outstanding

1. **Dependency monitoring.** Recheck Picomatch for a published fix, keep the pinned Node 22 patch current, and review the non-Windows Rust notices before adding targets. The SQLite/Vite/Node maintenance changes are described above.
2. **Refresh consistency.** Reproduce controlled out-of-order refresh responses, then address stale frontend results and consistent native read snapshots where needed.
3. **Preference recovery.** Validate parsed preference shapes (including valid JSON such as `null`) and provide a recoverable reset path. Parsing JSON alone is insufficient.
4. **Accessibility coverage.** Complete full-app Duplicates dialog and portalled picker focus checks, keyboard/screen-reader testing, contrast, themes and minimum-window coverage. The Quick preview focus fix does not resolve or verify every dialog.
5. **Representative performance profiling.** Measure 4,000/20,000-item startup, scrolling, cold/warm caches, rapid navigation/cancellation, native lock wait, database work, serialisation and React frame time. The repaired 1,200-item browser fixture is a synthetic development workload, not a production performance guarantee.
6. **Broader contracts and recovery acceptance.** Extend native-to-frontend contracts beyond the frozen won-lead fixture; strengthen combined duplicate removal/recovery/export/import, moved-library recovery, preference undo/restore and failure-repair scenarios. The browser suite still contains mocked backend logic and is not a substitute for native transaction tests.
7. **Maintainability.** Centralise reference remapping and timestamp/change contracts, extract refresh/history coordination from App, and reduce broad snapshot work where profiling or correctness evidence justifies it. These are targeted follow-ups, not a framework rewrite.
8. **Backup clarity and verification.** Add a user-facing coverage summary for exclusions such as My Business data, credentials, undo history and temporary queues, plus a disposable restore-verification command.

Standalone builds, installers, version bumps, tags and releases were explicitly excluded from these batches. A pushed development branch is a source backup; it does not update the standalone executable or imply integration into main.
