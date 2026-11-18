# Repeatable performance diagnostics

Diagnostics are opt-in and stop when the command finishes. They do not install a background service, copy the library, or attach to the user's browser. Reports stay under ignored `.tools/performance/` and must not be committed.

## Browser workload

Run `npm run diagnose:performance`. It starts an isolated headless Chrome and temporary Vite server, uses 1,200 synthetic thumbnails with varied proportions, scrolls, and repeatedly selects a thumbnail and opens/closes its Space preview. It checks for overlapping cards and records React commits/render time, ResizeObserver creation/callbacks, layout reads, thumbnail requests, peak mounted cards and browser long tasks. The server and temporary browser profile are cleaned up afterward. Avoid running other browser tests on port 1422 at the same time.

To keep named comparisons in PowerShell:

```powershell
$env:DESIGN_PERF_LABEL = 'before'
npm run diagnose:performance
$env:DESIGN_PERF_LABEL = 'after'
npm run diagnose:performance
Remove-Item Env:DESIGN_PERF_LABEL
```

Use the same machine, viewport, dataset, development mode and workload for comparison. Render and long-task timings vary with machine load. Native disk IO, real image decoding, network downloads and the user's library are not represented by this synthetic workload.

## Large-library profiling

`npm run diagnose:libraries` runs three isolated repetitions each at 4,000 and 20,000 items, sequentially at 1920 × 1080. Set `DESIGN_PERF_LABEL` to name a comparison and `DESIGN_PERF_REPETITIONS` (1–10) to change repetitions. For one run, set `DESIGN_PERF_ITEMS` to 1200, 4000 or 20000 and use `npm run diagnose:performance`. Reports include the checkout SHA/dirty flag, Node/Chrome/OS/CPU/memory, dataset size and mode. Raw reports remain ignored in `.tools/performance/`.

The workload records first-card latency from React mounting and separately from page navigation (including development module loading). It samples 24 positions across the full scroll range twice, switches between populated and empty categories six times, and opens/closes Space preview three times. It checks overlaps and a 250-card mounting ceiling. It records first/repeat-pass media requests, React render work, long tasks, animation-frame intervals and total navigation time. React render duration is not total frame time; animation-frame gaps include scheduling, rendering and other main-thread work. The fixed pauses and overlap check are included in elapsed workload time. The scroll positions are sampled jumps, not a continuous wheel gesture or a strict FPS benchmark.

Each repetition starts a fresh browser profile and Vite process. The first pass has a fresh application thumbnail cache; the second reuses that cache. The OS filesystem cache is **not** flushed. SVG data URLs represent varied proportions, not real image/video decoding or storage latency. Chrome runs headless with GPU disabled, and React runs in development mode. Do not interpret these numbers as desktop startup, GPU or release-build performance.

For native database and serialisation measurements:

```powershell
node scripts/desktop.mjs test synthetic_library_profile -- --ignored --nocapture
```

This opt-in test creates and deletes temporary SQLite libraries with 4,000/20,000 items, 40 projects, 40 tags, two memberships of each kind per item, categories and arrival timestamps. Four reads per size separate `db::load` from JSON encoding, validate loaded memberships and report payload bytes. A final read during an uncommitted WAL write validates snapshot isolation and measures read elapsed time. Every read opens a fresh SQLite connection, but OS pages are warm after seeding; this is not a cold-disk test. The test runs in the unoptimised debug test profile and excludes media, large history, business records and filesystem reconciliation. It prints only synthetic aggregate results.

`get_library` does not acquire the shared content write lock. The WAL-writer measurement is total read time, **not** an isolated mutex-wait measurement. For a real slow action, opt-in diagnostic phase events distinguish `native.library_worker_wait`, `native.library_read` and update lock phases. Collect those during a user-driven reproduction before attributing latency to lock contention. The refresh-consistency browser fixture and media-queue unit tests separately cover obsolete refresh results and queued cancellation; rapid navigation alone does not prove native decoder cancellation.

## Running desktop app

While using Design Dev, run `npm run diagnose:processes`. It samples the desktop app and its WebView descendants for 30 seconds. For a longer capture: `npm run diagnose:processes -- -Seconds 120` (maximum 300). Try the slow interaction during the capture. Reports include CPU percentage normalized across all logical processors and per-process working-set memory. They contain no filenames from the library, window titles, command lines or image data.

An idle sample cannot rule out delays during interaction. The debug build can also differ from an optimized standalone build. Capture the same interaction before and after a change; do not rebuild the standalone app solely to investigate a cosmetic tweak.

## 18 November 2026 profiling batch

Three browser repetitions per size, on Windows 10.0.26300, Intel i7-9750H (12 logical processors), 31.8 GiB RAM, Node 22.23.5 and Chrome 157.0.8103.78. Checkout: `71bb51c` plus this batch's profiling changes and the user's preserved working-tree edits. Vite development/headless CPU rendering; no production build. Browser and native profiling were run sequentially, but this was a shared desktop, not a controlled idle machine. Values below are medians (minimum–maximum), rounded to whole milliseconds where applicable.

| Measurement | 4,000 items | 20,000 items |
| --- | ---: | ---: |
| React mount → first card, ms | 1,553 (1,391–1,600) | 2,018 (1,858–2,093) |
| Page navigation → first card, ms | 40,564 (33,592–42,723) | 39,323 (30,629–42,407) |
| First scroll React work, ms | 4,206 (4,006–4,675) | 4,816 (3,962–5,040) |
| Repeat scroll React work, ms | 1,711 (1,543–2,333) | 2,417 (2,059–3,559) |
| First scroll frame interval p95, ms | 217 (200–217) | 267 (233–283) |
| Repeat scroll frame interval p95, ms | 150 (133–167) | 183 (150–233) |
| Six category round trips, ms | 1,605 (1,191–2,200) | 2,224 (2,132–3,535) |
| Peak mounted cards | 55 (55–55) | 56 (55–56) |

All six runs passed overlap, bounded mounting, rapid navigation and repeated Space-preview checks. Fresh Vite module loading dominates the navigation-to-first-card measure; this is not the persistent Design Dev startup time. Mounted cards remain bounded as the dataset grows fivefold. Repeat scrolling reduces React work, but substantial frame gaps remain. Because the scroll visits only 24 positions and image dimensions settle during the run, the repeat pass can request additional thumbnails; it is not a fully predecoded library.

Native debug test, four reads per size:

| Measurement | 4,000 items | 20,000 items |
| --- | ---: | ---: |
| Database load median (range), ms | 480 (358–490) | 763 (494–1,119) |
| JSON encoding median (range), ms | 595 (515–662) | 1,114 (837–1,841) |
| JSON bytes | 1,508,691 | 7,570,774 |
| Read during open WAL writer, ms (one sample) | 416 | 1,054 |

The writer test completed before rollback and returned the committed item name. Encoding is a material debug-build cost, and the payload grows with library size. These measurements do not establish production bottleneck percentages: native and browser workloads differ, IPC is mocked in the browser, and there is no end-to-end native-to-WebView trace. No runtime optimisation was made on this evidence alone.

Next performance work should start with a user-driven slow interaction and its diagnostic phases, then compare before/after under the same conditions. Remaining acceptance includes cold-disk startup, real thumbnail/video cache misses and decoding, native worker/lock contention, cancellation during native work, and a representative library containing history/business data. Do not flush system caches, copy private library data or build a release merely to manufacture a comparison.

Validation: six large-library browser profiles, the explicit synthetic native profile, frontend/unit and native regression suites, TypeScript and the refresh-consistency browser fixture. The user confirmed normal library opening, gallery scrolling, rapid category switching and Space → Escape → Space in Design Dev. Raw timings remain in ignored local reports; no library data was read or changed by the profiling fixtures.

## November 2026 investigation

The original masonry virtualizer multiplied overscan by the number of lanes after our code had already scaled it by columns. It also recreated observers and repeatedly read the bounds of all mounted tiles. Static image hover updated app-wide state unnecessarily.

The media grid now calculates non-overlapping positions from cached image proportions, batches image-size updates once per animation frame, and mounts only thumbnails near the visible viewport. Resizing recalculates positions without remounting the grid. Still-image hover uses CSS; motion previews retain hover state. Image dimensions are cached in memory with a bounded entry count, without storing user paths or image bytes. All media categories and project galleries use this layout; thumbnail link boards use responsive CSS columns.

The current workload uses gallery Space preview; the historical runs below used the older click-to-open side preview. Their numbers are historical evidence, not a directly comparable baseline for the revised workload.

The two retained reports each record one run on 3 November 2026 UTC at 1920 × 1080, using headless Chrome and the Vite development server. The gallery change landed in `d4e7673` (parent `86546f4`). The reports did not record the tested checkout SHAs, machine specifications or Chrome version. For future comparisons, record those details and the number of repetitions alongside each result.

| Measurement | Before | After |
| --- | ---: | ---: |
| Peak mounted cards | 228 | 56 |
| Bounding-rectangle reads | 13,777 | 225 |
| ResizeObservers created | 109 | 4 |
| React render time | 7,067 ms | 1,890 ms |
| Thumbnail requests | 228 | 117 |
| Long-task time | 11,304 ms | 1,656 ms |

Overlap checks, Space-preview return/focus, full-viewer breakpoints, link-board interactions and drag operations should accompany future gallery changes. `npm run test:grid`, `npm run test:preview`, `npm run test:boards` and `npm run test:drag` cover those workflows. Run checks using port 1421 sequentially; the side-preview check uses port 1422.
