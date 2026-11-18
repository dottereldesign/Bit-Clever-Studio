# Native contract fixture

`native-won-lead.json` is unmodified `serde_json::to_vec(&db::load(...))` output captured during the 2026-11-18 audit from a temporary SQLite database. It contains only synthetic data and built-in default tags; no user library was read.

The producer initialised an empty database with `db::init`, then called `db::change` twice with `action: "update_studio"`: first `{id: "fixture", patch: {kind: "lead", name: "Fixture", status: "new", currency: "NZD", value: 1200}}`, then `{id: "fixture", patch: {status: "won"}}` as the JSON `value`. Finally it serialised `db::load(&path, false)`. Native creation/close timestamps and undo entries are preserved exactly.

The frontend test anchors its clock to the emitted close date, so it remains valid after the capture month. This frozen contract guards against reintroducing seconds-based frontend assumptions. It does not replace native persistence tests or automatically detect every future schema change. Regenerate from a disposable native database when the wire schema intentionally changes; never substitute a real library export.


## Recovery and import contract

`native-recovery.json` is produced by `recovery_contract_tests::recovery_contract_round_trip` in Rust. The test creates a disposable managed SQLite library and two tiny generated images, merges a duplicate's memberships/references, moves the library, exercises a failed restore collision and successful recovery, exports, relocates the backup and imports into a database with colliding IDs. It captures `db::load` snapshots after import and rating/undo/redo, plus `restore_preferences` output. Only temporary filesystem prefixes are replaced with `C:/SyntheticContract/`; timestamps, IDs, nulls, arrays and native wire field names are preserved.

`npm test` consumes the frozen fixture on any platform. `npm run test:contracts` runs the Rust producer and then the same frontend consumer assertions against its **fresh** output in ignored `.tools/contracts/native-recovery.json`. The runner removes the previous output first and stops if either side fails. Windows CI runs this live contract in addition to the full native suite, so a stale frozen fixture cannot hide producer/consumer incompatibility.

The frontend assertions cover design/component/project/folder membership indexes, nested folders, remapped moodboard/client attachments, project-only links, ratings, durable history fields and remapped preferences through normalisation and local Undo/Redo. The native scenario also checks import transaction rollback, occupied recovery-path preservation, preference persistence failure/retry, derivative-file warning/repair and rejected incomplete activation preserving the active-library pointer.

To intentionally refresh the frozen fixture, run `npm run test:contracts`, inspect the synthetic output, then copy `.tools/contracts/native-recovery.json` to `tests/fixtures/native-recovery.json` and run `npm test`. Never use a private library export. This covers selected contracts, not every native command or abrupt process/power loss during filesystem operations.
