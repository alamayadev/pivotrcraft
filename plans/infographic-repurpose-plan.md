# VectorCraft Infographic Repurpose Plan

> **Status:** Brainstorming / pre-fork planning  
> **Date:** 2026-11-26  
> **Scope:** Repurpose VectorCraft into an infographic generator that creates ready-to-use vector charts and custom decorative SVG assets from CSV data, while keeping upstream sync feasible.

## 1. Executive Summary

VectorCraft already provides most low-level primitives needed for an infographic workflow:

- **Vector graphs/charts:** 9 graph kinds, editable graph spec/data, design/marker customization (`crates/doc/src/graph.rs`, `crates/engine/src/cmd/graph/*`).
- **CSV/data paths:** graph CSV parsing/apply, dataset CSV import/export, graph data file import (`crates/engine/src/cmd/graph/data.rs`, `crates/engine/src/cmd/variablecmds/csv.rs`).
- **SVG/vector import:** SVG/SVGZ open/import/place as editable vector groups (`crates/engine/src/cmd/fileio.rs`, `crates/engine/src/cmd/place.rs`).
- **Reusable assets:** symbols, swatches, graphic styles, brushes, templates, libraries.
- **Automation surface:** command registry + MCP/control protocol (`crates/engine/src/cmd/*`, `docs/mcp.md`, `crates/mcp/*`).
- **Custom chart shapes:** `graph.design` + `graph.column` already support `vertical|uniform|repeating|sliding` column designs and guides, which can express nontraditional bar silhouettes. [`crates/engine/src/cmd/graph/commands.rs:45`](crates/engine/src/cmd/graph/commands.rs:45)

The repurpose should be implemented as an **additive product layer** over these systems. Avoid forking core behavior unless necessary; this minimizes merge conflicts when syncing from `storytold/vectorcraft`.

## 2. Goals

| Goal | Description |
|------|-------------|
| G1 | Generate infographic-ready vector charts from CSV data through commands and UI. |
| G2 | Allow users to add ready-to-use decorative/custom SVG assets to the workspace. |
| G3 | Provide reusable infographic templates/styles for consistent output. |
| G4 | Keep output editable as normal VectorCraft vector objects. |
| G5 | Preserve upstream-sync viability by isolating new behavior and adding regression tests. |

## 3. Non-Goals (Initial Repurpose)

- Rewriting VectorCraft into a separate non-vector app.
- Replacing existing graph engine with a new chart engine.
- Cloud-only SaaS workflow; initial focus is local document/workspace generation.
- AI-generated decorative assets in MVP.
- Full BI/dashboard system (live DB connections, collaborative web dashboards).

## 4. Upstream Sync Strategy

Current repo state: `origin=https://github.com/storytold/vectorcraft.git`, branch `main`. Before implementation, fork and add remotes:

```powershell
git remote rename origin upstream
git remote add origin <fork-url>
git checkout -b product/infographic
```

### Sync Rules

1. **Additive-first:** add new `infographic.*` commands, new modules, new asset packs, and new UI panels/dialogs.
2. **Thin integration points:** register commands in existing registries with small diffs; avoid reformatting unrelated code.
3. **No default behavior breakage:** existing `graph.*`, `file.place`, CSV/dataset, and SVG import behavior must remain compatible.
4. **Feature flags/config:** gate major infographic UI entry points behind config/prefs if needed during development.
5. **Conflict hotspot awareness:** expect conflicts in `README.md`, `docs/*`, command registries, menus, graph/variable/place code.
6. **Sync cadence:** fetch upstream regularly; merge/rebase on short intervals; resolve conflicts in small batches.
7. **Tests as merge armor:** every infographic capability gets command-level tests so upstream changes cause explicit failures.

## 5. Architecture Principles

- **Command-driven:** all infographic behavior exposed as engine commands; UI/MCP call commands (AGENTS rule).
- **Editable output:** generated charts/decorations remain normal nodes/groups/text/graphs, not opaque blobs.
- **Deterministic generation:** same CSV + template + options => same layout/result, except explicit randomness options.
- **Provenance-safe assets:** every bundled decorative SVG has license/source record in `ASSETS.md` and `licenses/` as required.
- **No handwritten JS/TS:** follow Rust-only app rule.
- **Security/robustness:** treat CSV/SVG/template inputs as untrusted; size limits, parse errors, no panic on bad input.
- **Multi-source composition:** one infographic page/artboard can contain multiple charts from different CSV sources, plus multiple decorative/object layers; each chart keeps its own data mapping and preset metadata.

## 6. Proposed Target UX

### MVP User Flow

1. User selects **File > New Infographic from CSV** or opens Infographic panel.
2. User picks CSV file/data.
3. User maps columns: label/category, value(s), series/group, optional annotations.
4. User chooses chart type and style preset.
5. User optionally selects decorative SVG pack/items.
6. Generator creates an artboard/group containing chart, title, labels, legend, and decorations.
7. User edits generated vector objects normally; can re-run generation with updated options.

### One-Page, Multi-Source Infographic Workspace

An infographic page is a composition, not a single chart. The right-side Infographic panel should manage the active infographic document/artboard and allow multiple data sources and object layers.

| Tab | Purpose | Primary actions |
|-----|---------|-----------------|
| **Charts** | Manage chart blocks and CSV data sources | Add chart from CSV, duplicate chart, pick CSV, map columns, choose preset (`bar-building`, `bar-arrow-highlight`), set axis/legend/highlight defaults, replace data, remove chart. |
| **Objects** | Manage decorative/custom SVG and reusable objects | Browse ready-to-use SVG/icon packs, place object, scale/rotate/anchor, recolor where allowed, save selection as reusable object. |
| **Text** *(MVP or Phase 4)* | Style infographic text using existing VectorCraft text tools | Shortcuts/embedded controls for Character/Paragraph/OpenType/Type on Path/Area Type; preset text styles for title, subtitle, data label, annotation, source note. No duplicate text engine. |
| **Layout** *(MVP or Phase 4)* | Arrange infographic blocks | Align/distribute chart blocks, snap to slots/grid, spacing, grouping, artboard fit; reuse existing align/transform commands. |
| **Future tabs** | Extensible object/data modules | Maps, icons/data badges, timelines, illustrations, AI-assisted suggestions (future, not MVP). |

**Panel behavior:**
- **Context-aware:** when an infographic chart is selected, Charts tab shows that chart's CSV source, mapping, preset, and style options; when text is selected, Text tab surfaces relevant text styling shortcuts; otherwise it shows page-level controls.
- **Reuse existing text styling:** Text tab is a dedicated infographic shortcut surface, not a new text system. It should invoke existing text commands/panels (Character, Paragraph, Properties, text styles, text tool actions) and expose infographic-friendly presets.
- **Multiple CSV sources:** each chart block stores its own source reference (`path`, embedded data snapshot, or dataset id) and mapping; one page can contain several charts from different CSV files.
- **Non-destructive:** changing a chart preset or mapping updates that chart block; other charts/objects remain untouched.
- **Editable output:** generated charts and decorations remain normal VectorCraft objects/groups; panel actions issue commands.

### Custom Chart Style Direction

Infographic charts prioritize visual clarity over dense analytical axes. The first custom chart MVP should ship **ready-to-use style presets** rather than requiring users to author graph designs manually.

| Preset ID | Visual idea | Implementation strategy | Notes |
|-----------|-------------|-------------------------|-------|
| `bar-building` | Tall building-like bar with simple windows/detail near the top, two-color theme per bar item | `graph.design` + `graph.column` with `sliding` mode; horizontal guide separates fixed top/cap from stretchable body | Keep roof/window detail above guide to avoid distortion. Two-color theme is baked into preset art; optional recolor variants can be added later. |
| `bar-arrow-highlight` | Regular bar chart, but last bar or tallest bar has an arrow head/cap | Create normal bar chart via `graph.create/setData/setType`, then infographic post-processing overlays/replaces the target bar with an arrow-cap vector | `graph.column` applies to series, not conditional items; conditional highlight is generator logic over editable output. |

### Infographic Chart Style Defaults

Custom infographic presets should use minimal chart chrome by default:

- **Y-axis:** `minimal` by default. Use `tickLength: "none"`, `tickMarks: 1`, no prefix/suffix, and no heavy grid emphasis. A `hidden` mode may remove/hide generated value labels as post-processing because `graph.setType` has no explicit value-label visibility field.
- **X-axis:** bold category labels by default for readability. Apply through label style post-processing (`LabelStyles.categories`) or equivalent generated text styling.
- **Legend:** optional; off for single-series infographic bars unless user enables it.
- **Highlight rule:** `none | last | max | min | index` for conditional styles such as `bar-arrow-highlight`.
- **Output:** all generated bars, caps, labels, and decorations remain normal editable VectorCraft objects.

### MCP/CLI Flow

```json
{"name":"run_command","arguments":{"command":"infographic.addChartFromCsv","params":{
  "infographicId":"info-1",
  "csvPath":"sales.csv",
  "mapping":{"category":"Month","values":["Sales"]},
  "stylePreset":"bar-building",
  "axis":{"y":"minimal","xLabelStyle":"bold"},
  "placement":{"slot":"chart-2","x":960,"y":180,"width":800,"height":500}
}}}
```

## 7. Phase Plan

## Phase 0 - Discovery, Fork Baseline, and Design Constraints

**Objective:** establish a fork-safe baseline and lock technical/product constraints before coding.

**Work items:**
- Create fork remote/branch strategy; document sync workflow.
- Inventory existing graph, CSV, dataset, SVG place, symbol/style/template systems.
- Identify integration points: command registry, UI menus/panels, MCP docs/schema, file format implications.
- Define infographic MVP scope and non-goals.
- Define asset licensing/provenance checklist for decorative SVGs.
- Create ADR-style notes in `plans/` for key decisions (additive module, command naming, template format).

**Deliverables:**
- `plans/` planning docs (this file + optional ADRs).
- Baseline checklist for upstream sync.
- MVP capability matrix.

**Acceptance criteria:**
- [ ] Fork/sync workflow documented with exact git commands.
- [ ] Existing reusable primitives mapped to file paths.
- [ ] MVP/non-goal boundary approved.
- [ ] Asset provenance rules documented, including `ASSETS.md`/license requirements.
- [ ] No product code changes required in this phase.

## Phase 1 - CSV Ingestion and Data Mapping

**Objective:** provide robust CSV import/mapping for infographic generation, reusing existing CSV parsers where possible.

**Work items:**
- Add `infographic.inspectCsv` / `infographic.previewCsv` command:
  - reads CSV/TSV/text input (path/data/base64) with existing safe limits.
  - returns columns, sample rows, inferred types, warnings.
- Add mapping schema:
  - `category`, `series`, `value`/`values`, optional `label`, `color`, `icon`, `note`.
  - aggregation options: none/sum/avg/count/min/max.
  - missing-value policy: skip, zero, error.
- Implement validation with clear errors and no mutation on failure.
- Reuse `graph_csv_from_file`/dataset CSV parsing behavior where practical; do not change existing command semantics.

**Likely touch points:**
- New: `crates/engine/src/cmd/infographic/*` or `crates/engine/src/cmd/infographic.rs`.
- Reuse: `crates/engine/src/cmd/graph/data.rs`, `crates/engine/src/cmd/variablecmds/csv.rs`.
- Tests: `crates/engine/tests/*` or module tests near infographic commands.

**Acceptance criteria:**
- [ ] Can inspect a CSV and return column metadata/sample rows.
- [ ] Mapping validation catches missing columns, invalid numeric values, unsupported aggregations.
- [ ] Missing-value policies behave as specified and are tested.
- [ ] Large/invalid/untrusted CSV inputs fail gracefully with limits/errors.
- [ ] Existing `graph.create`, `graph.setData`, `dataset.import/export` tests still pass.

## Phase 2 - Chart Presets and Infographic Primitives

**Objective:** convert mapped data into editable chart primitives using existing graph engine, starting with two custom infographic bar styles, and support multiple chart blocks per infographic page.

**Work items:**
- Add `infographic.createChart` command:
  - inputs: mapped data, chart kind, dimensions, position/artboard target, style preset, axis style, highlight rule.
  - output: graph node id, generated group ids, applied preset metadata.
- Add multi-chart commands:
  - `infographic.addChartFromCsv`: create a chart block from CSV mapping and place it into an infographic page/group.
  - `infographic.updateChart`: change selected chart's CSV source, mapping, preset, axis, or highlight rule without affecting unrelated charts.
  - `infographic.listCharts`: inspect chart blocks in the active infographic/page.
- Implement first two custom bar presets:
  - `bar-building`: custom building silhouette bar design (simple windows/detail near top, two-color theme) using `graph.design` + `graph.column` `sliding` mode with a horizontal guide.
  - `bar-arrow-highlight`: regular bars with conditional arrow head/cap on `last|max|min|index` target; implemented as graph generation + post-processing overlay/replacement.
- Add infographic chart style defaults:
  - Y-axis `minimal` default: `tickLength: "none"`, `tickMarks: 1`, no prefix/suffix; optional hidden value labels via post-processing.
  - X-axis category labels bold by default.
  - Legend off by default for single-series presets; configurable.
- Keep preset application additive:
  - reuse `graph.create`, `graph.setData`, `graph.setType`, `graph.design`, `graph.column` where possible.
  - post-processing only creates/edits normal vector/text nodes; no opaque rendering.
- Add preset registry/manifest (Rust code or asset-backed manifest) with provenance for any bundled design SVG/vector art.

**Likely touch points:**
- `crates/engine/src/cmd/graph/*` for reusable helpers only if needed.
- New infographic preset definitions (Rust structs/serde, or asset-backed templates).
- New infographic command module for preset application/post-processing.
- Docs: command params in code + `docs/mcp.md` update later.

**Acceptance criteria:**
- [ ] CSV-mapped data creates a chart using existing graph engine.
- [ ] `bar-building` preset renders a building-style bar shape via sliding column design; top detail does not distort when values change.
- [ ] `bar-arrow-highlight` preset renders regular bars and applies arrow cap to configured target (`last` and `max` at minimum).
- [ ] Default infographic style applies minimal Y-axis and bold X-axis category labels; user can override.
- [ ] One infographic page can contain multiple chart blocks sourced from different CSV files/mappings.
- [ ] Updating one chart block does not mutate unrelated chart blocks unless explicitly requested.
- [ ] Generated chart remains editable with existing graph commands/tools; conditional overlays are editable vector objects.
- [ ] Invalid mapping/preset/highlight rule returns error; no document mutation on failure.

## Phase 3 - Decorative SVG Asset Library

**Objective:** make custom decorative SVGs first-class, ready-to-use workspace assets.

**Work items:**
- Define decorative asset packaging:
  - built-in library under `assets/` or dedicated infographic asset directory.
  - metadata: name, tags, category, license, source, recommended size/color behavior.
- Add `infographic.listDecorations` / `infographic.placeDecoration` commands.
- Reuse `file.place` / SVG import path; normalize placement (center/anchor, scale, optional recolor).
- Add optional recolor strategy:
  - preserve multi-color art by default.
  - optional single-tone/brand tint mode for compatible SVGs.
- Establish provenance workflow:
  - update `ASSETS.md`.
  - include license texts in `licenses/` or asset-local license files as repo convention requires.
  - only assets with compatible licenses and clear provenance.

**Likely touch points:**
- `assets/` new infographic decorative pack.
- `ASSETS.md`, `licenses/`, possibly `NOTICE`.
- `crates/engine/src/cmd/place.rs` / file loading reuse.
- UI asset browser later.

**Acceptance criteria:**
- [ ] User/agent can list available decorative SVG assets.
- [ ] Selected decorative SVG can be placed into active document/artboard at requested position/size.
- [ ] Placement uses existing SVG import behavior; imported objects are editable vectors.
- [ ] Every bundled decorative asset has provenance and license entries.
- [ ] No incompatible/unattributed asset is added.

## Phase 4 - Infographic Layout and Template Composition

**Objective:** compose chart blocks + text + decorations into infographic templates, supporting multiple CSV-driven charts on one page.

**Work items:**
- Define template schema:
  - canvas/artboard size, margins, grid.
  - slots: title, subtitle, chart slots (one or many), legend, source note, icon/decorations.
  - style tokens: palette, fonts, stroke/fill rules.
- Add `infographic.createFromCsv` command:
  - end-to-end: inspect/map/validate -> create chart -> place decorations -> add text -> group/name.
  - supports initial chart block; additional charts added through `infographic.addChartFromCsv`.
- Add template selection and parameter overrides.
- Support deterministic placement anchors (top-left, center, align to artboard, slot id, etc.).
- Reuse existing text styling system for infographic text:
  - generated title/subtitle/annotation/source text are normal text objects.
  - Text tab provides shortcuts to existing Character/Paragraph/Properties/text style commands and panels; no parallel text styling implementation.
- Make generated output grouped/labeled for easy editing; each chart block gets stable metadata/id for panel updates.
- Consider data-driven variants later using existing Variables/datasets pattern; MVP can generate one composition per run, with multiple chart blocks inside it.

**Likely touch points:**
- New infographic template module/assets.
- Reuse `text.create`/text commands, `file.place`, `graph.create/setData/setType`.
- Reuse existing text/style UI and commands: `crates/ui-egui/src/panels/character.rs`, `paragraph.rs`, `text_styles.rs`, `properties.rs`, `crates/engine/src/cmd/typecmd.rs`, `textstyles.rs`.
- Possibly `object.group`, `transform`, alignment commands.

**Acceptance criteria:**
- [ ] One command can generate a complete infographic draft from CSV + template + options.
- [ ] Output contains editable chart, text, and decorative SVG elements.
- [ ] Layout supports multiple chart slots/blocks on one page with deterministic placement.
- [ ] Layout respects template margins/grid/anchors across supported artboard sizes.
- [ ] Text tab actions reuse existing VectorCraft text styling commands/panels; no duplicate text engine is introduced.
- [ ] Generation is deterministic given same inputs/options.
- [ ] Failures are atomic where possible: no partial broken document state beyond a clear undo step/policy.

## Phase 5 - UI, MCP, and Workflow Integration

**Objective:** expose infographic generation in UI and automation surfaces without disrupting core UX.

**Work items:**
- Add UI entry points:
  - File menu: "New Infographic from CSVâ€¦" / "Generate Infographicâ€¦".
  - Infographic panel/dialog for CSV mapping, template, preset, decorations.
- Add right-side Infographic panel with extensible tabs:
  - **Charts tab:** chart block list, "Add chart from CSV", per-chart source/mapping/preset/axis/highlight controls, update/remove actions.
  - **Objects tab:** ready-to-use SVG/icon/decorative object browser, place/scale/anchor/recolor controls.
  - **Text tab (MVP/Phase 4):** infographic text presets and shortcuts to existing VectorCraft text styling tools (Character/Paragraph/Properties/text styles); selecting text focuses relevant controls.
  - **Layout tab (MVP/Phase 4):** alignment, spacing, grid/slot snapping, group/lock/order shortcuts for infographic blocks.
  - Tab registry designed for future modules (maps, badges, timelines) without redesigning the panel shell.
- Add command palette/menu registrations with minimal diffs.
- Expose commands through MCP/`run_command` docs.
- Add previews/thumbnails for templates and decoration packs.
- Add i18n strings through existing localization system (English source keys only in feature PR).
- Accessibility: keyboard flow, labels, focus, screen-reader names for new controls.

**Likely touch points:**
- `crates/ui-egui/src/menus.rs`, panel modules under `crates/ui-egui/src/panels/*`.
- `crates/ui-egui/src/state.rs`, `crates/ui-egui/src/panel_docking.rs`, `crates/ui-egui/src/panels.rs` for panel registration/flyout behavior.
- Reuse existing text panels/commands instead of duplicating styling logic: `panels/character.rs`, `panels/paragraph.rs`, `panels/text_styles.rs`, `panels/properties.rs`, `cmd/typecmd.rs`.
- `crates/engine/src/cmd/*` registration.
- `docs/mcp.md`, `docs/gaps.md`/roadmap docs only if project process requires; keep doc updates scoped.

**Acceptance criteria:**
- [ ] User can generate infographic from UI without using MCP.
- [ ] Infographic panel provides Charts, Objects, and Text tabs (Layout optional in same milestone); Charts tab supports multiple charts from different CSV sources on one infographic page.
- [ ] Selecting a chart block shows that chart's data source/mapping/preset options; editing it does not affect unrelated charts.
- [ ] Text tab reuses existing VectorCraft text styling functions/panels; no duplicate text styling engine or divergent text behavior.
- [ ] Panel/tab architecture allows adding a future tab without breaking existing tab state or command contracts.
- [ ] All UI actions call engine commands; no UI-only state mutation for document changes.
- [ ] MCP docs include new commands, params, examples, and error behavior.
- [ ] Keyboard/accessibility smoke checks pass for new dialog/panel.
- [ ] Existing menus/commands remain functional.

## Phase 6 - Export, Batch, QA, and Release Hardening

**Objective:** make infographic output production-ready and safe for upstream merges.

**Work items:**
- Export presets for infographic outputs: PNG, SVG/SVGZ, PDF, JPG/WebP as supported.
- Optional batch generation:
  - multiple CSVs -> multiple infographics/files.
  - dataset rows -> variants (evaluate reuse of Variables/datasets).
- Performance checks for large CSVs and decoration-heavy templates.
- Fuzz/robustness tests: malformed CSV, malformed SVG, huge values, missing fonts, missing assets.
- Documentation: user guide, MCP examples, template authoring guide, asset provenance guide.
- Upstream sync rehearsal: merge latest upstream into product branch, resolve conflicts, run checks.

**Likely touch points:**
- `crates/engine/src/cmd/fileio.rs`, export commands, screens export.
- tests in `crates/engine/tests`, `crates/svg/tests`, format tests as needed.
- `docs/` new infographic docs.

**Acceptance criteria:**
- [ ] Generated infographic exports correctly to PNG/SVG/PDF with visual smoke checks.
- [ ] Batch/variant flow (if included) produces deterministic named outputs.
- [ ] Malformed inputs fail with actionable errors; no crashes/panics.
- [ ] Relevant crate tests/clippy/fmt pass; `cargo xtask ci` or agreed subset passes.
- [ ] Upstream merge rehearsal completed with documented conflict resolutions.

## 8. Cross-Cutting Acceptance Criteria

These apply to every phase that changes behavior:

- [ ] **Command-driven:** document mutations go through engine commands.
- [ ] **Editable output:** generated items remain normal VectorCraft objects.
- [ ] **No silent degradation:** unsupported/partial behavior reports warnings/errors clearly.
- [ ] **Provenance:** all bundled assets have source/license attribution.
- [ ] **Compatibility:** existing graph/place/file workflows unaffected.
- [ ] **Tests:** unit/integration tests cover success, failure, and edge cases.
- [ ] **Docs:** MCP/command docs updated for new public commands.

## 9. Suggested Command Surface (Draft)

| Command | Purpose | Phase |
|---------|---------|-------|
| `infographic.inspectCsv` | Parse CSV, return columns/types/sample/warnings. | 1 |
| `infographic.mapData` | Validate mapping and produce normalized dataset for charts. | 1 |
| `infographic.listChartPresets` | List built-in/custom chart presets, defaults, and preview metadata. | 2 |
| `infographic.createChart` | Create chart from mapped data + preset. | 2 |
| `infographic.addChartFromCsv` | Add a new chart block to an infographic page from a CSV source. | 2 |
| `infographic.updateChart` | Update a selected chart block's source/mapping/preset/style. | 2 |
| `infographic.listCharts` | List chart blocks, sources, mappings, and preset metadata for active infographic/page. | 2 |
| `infographic.listTemplates` | List available infographic templates. | 4 |
| `infographic.listDecorations` | List decorative SVG/symbol assets. | 3 |
| `infographic.placeDecoration` | Place/recolor/scale decoration. | 3 |
| `infographic.createFromCsv` | End-to-end infographic generation. | 4 |
| `infographic.export` | Export generated infographic with preset. | 6 |

> Final names/params should follow existing command naming conventions and schema style before implementation.

## 10. Data Model Draft

### CSV Mapping

```json
{
  "category": "Month",
  "values": ["Revenue", "Profit"],
  "series": "Region",
  "ignoreRows": [],
  "aggregation": "none",
  "missing": "skip"
}
```

### Chart Preset Definition

```json
{
  "id": "bar-building",
  "kind": "bar",
  "label": "Building Bars",
  "design": {
    "source": "assets/infographic/chart-designs/bar-building.svg",
    "apply": "column",
    "columnType": "sliding",
    "guide": "horizontal-top-cap"
  },
  "style": {
    "palette": ["#1E88E5", "#90CAF9"],
    "axis": {
      "y": "minimal",
      "xLabelStyle": "bold",
      "legend": "off"
    }
  },
  "highlight": {
    "mode": "none"
  }
}
```

### Conditional Highlight Preset

```json
{
  "id": "bar-arrow-highlight",
  "kind": "bar",
  "base": "bar-regular",
  "axis": {"y": "minimal", "xLabelStyle": "bold"},
  "highlight": {
    "mode": "max",
    "shape": "arrow-cap",
    "target": "value-bar"
  }
}
```

### Infographic Document Model

```json
{
  "id": "infographic-1",
  "artboardId": 3,
  "charts": [
    {
      "id": "chart-1",
      "source": {"kind": "csvPath", "path": "sales-q1.csv"},
      "mapping": {"category": "Month", "values": ["Sales"]},
      "preset": "bar-building",
      "axis": {"y": "minimal", "xLabelStyle": "bold"},
      "highlight": {"mode": "none"},
      "nodeId": 1024
    },
    {
      "id": "chart-2",
      "source": {"kind": "csvPath", "path": "profit-q1.csv"},
      "mapping": {"category": "Month", "values": ["Profit"]},
      "preset": "bar-arrow-highlight",
      "axis": {"y": "minimal", "xLabelStyle": "bold"},
      "highlight": {"mode": "max"},
      "nodeId": 1180
    }
  ],
  "objects": [
    {"id": "obj-1", "asset": "infographic-icons/trend-up", "nodeId": 1201}
  ]
}
```

### Template Definition

```json
{
  "id": "bar-report-basic",
  "size": {"width": 1920, "height": 1080},
  "slots": {
    "title": {"x": 80, "y": 64, "width": 1200},
    "chart": {"x": 80, "y": 180, "width": 1200, "height": 700},
    "decorations": [{"anchor": "top-right", "asset": "accent-shape"}]
  },
  "style": {"palette": "brand-a", "font": "Inter"}
}
```

## 11. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Upstream merge conflicts in core registries | Slows sync | Additive modules, tiny registry diffs, frequent upstream merges. |
| CSV ambiguity (types, locale, missing values) | Wrong charts | Explicit mapping UI, preview, strict validation, clear warnings. |
| Decorative SVG licensing/provenance gaps | Legal/repo policy risk | Asset checklist, `ASSETS.md` updates, license files, no unattributed assets. |
| Generated layouts look poor on varied data | Low product value | Preset constraints, auto-scaling rules, template QA fixtures. |
| Performance on large CSV/decorations | Bad UX | Input limits, streaming/efficient parse, progress reporting, benchmarks. |
| Fork divergence becomes heavy | Maintenance burden | Treat infographic as layer; avoid renaming/replacing core systems in MVP. |

## 12. Testing Strategy

- **Unit tests:** CSV parsing/mapping, preset resolution, placement math, template schema validation.
- **Command tests:** `infographic.*` commands success/failure/no-mutation-on-error.
- **Integration tests:** CSV -> infographic -> export smoke fixtures.
- **Regression tests:** existing `graph.*`, `dataset.*`, `file.place` SVG flows unchanged.
- **Robustness tests:** malformed CSV/SVG, oversized inputs, invalid UTF-8, hostile metadata.
- **Manual QA:** UI keyboard flow, accessibility labels, visual output review.

## 13. Implementation Order Recommendation

1. **Phase 0/1 first:** data mapping correctness is the foundation.
2. **Phase 2 next:** implement `bar-building` and `bar-arrow-highlight` as the first two custom infographic chart presets, including minimal-Y/bold-X defaults.
3. **Phase 3/4:** add decorations and composition templates around the chart presets.
4. **Phase 5:** UI/MCP polish after command surface stabilizes.
5. **Phase 6:** harden exports, batch, sync rehearsal.

## 14. Definition of Done (MVP)

The repurpose MVP is done when:

- [ ] A user can provide CSV data, map columns, pick a template/preset, and generate an editable infographic in VectorCraft.
- [ ] One infographic page can contain multiple chart blocks from different CSV sources, managed from the right-side Charts tab.
- [ ] The first two custom bar presets are available: `bar-building` (sliding building silhouette, two-color theme) and `bar-arrow-highlight` (regular bars with conditional last/max arrow cap).
- [ ] Infographic chart defaults support minimal Y-axis and bold X-axis labels, with user override.
- [ ] The user can place ready-to-use decorative SVG assets from a curated, licensed library through the Objects tab/panel flow.
- [ ] The Infographic Text tab provides shortcuts to existing VectorCraft text styling functions; generated/edited text uses normal text objects and commands.
- [ ] The generated result exports to PNG/SVG/PDF.
- [ ] Commands are documented and available to MCP/automation.
- [ ] Existing VectorCraft core workflows remain intact.
- [ ] Upstream sync process is documented and rehearsed.
