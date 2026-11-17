# Native contract fixture

`native-won-lead.json` is unmodified `serde_json::to_vec(&db::load(...))` output captured during the 2026-11-18 audit from a temporary SQLite database. It contains only synthetic data and built-in default tags; no user library was read.

The producer initialised an empty database with `db::init`, then called `db::change` twice with `action: "update_studio"`: first `{id: "fixture", patch: {kind: "lead", name: "Fixture", status: "new", currency: "NZD", value: 1200}}`, then `{id: "fixture", patch: {status: "won"}}` as the JSON `value`. Finally it serialised `db::load(&path, false)`. Native creation/close timestamps and undo entries are preserved exactly.

The frontend test anchors its clock to the emitted close date, so it remains valid after the capture month. This frozen contract guards against reintroducing seconds-based frontend assumptions. It does not replace native persistence tests or automatically detect every future schema change. Regenerate from a disposable native database when the wire schema intentionally changes; never substitute a real library export.
