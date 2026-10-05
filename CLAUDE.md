# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Kinaro is a native desktop REST API testing/exploration tool, written in Rust and built on [GPUI](https://github.com/zed-industries/zed) (via the `gpui-kit` crate — Zed's GPU-accelerated UI framework). Projects are organized as Test Suites → Test Cases → Test Steps, persisted locally in a diff-friendly file format.

## Commands

```bash
cargo check                       # type-check the whole workspace
cargo clippy                      # lint (also run in CI)
cargo test --workspace            # run all tests (also run in CI)
cargo test -p ki_workspace        # run tests for a single crate
cargo test -p ki_workspace test_case::   # run tests matching a path/module (substring match)
cargo test -p ki_workspace some_test_name -- --exact  # run a single test by name
cargo run -p kinaro                # run the app (default workspace member is crates/app)
cargo build --release
cargo fmt                          # formatting is enforced via rustfmt.toml (2-space indent, 130 col width, edition 2024)
```

Linux builds need `libxkbcommon-dev`, `libwayland-dev`, `libfontconfig1-dev`, a Vulkan driver, etc. — see `script/install-linux.sh` (this is what CI installs on Ubuntu runners). Windows needs MSVC Build Tools (`script/install-window.ps1`).

CI (`.github/workflows/ci.yml`) runs `cargo check`, `cargo clippy`, and `cargo test --workspace` on Linux and Windows for every PR/push to `main`.

## Workspace layout

Cargo workspace with `crates/app` as the only default member (binary crate name: `kinaro`). Internal crates are referenced by their `ki_*` package names via `workspace.dependencies` in the root `Cargo.toml`:

- **`crates/app`** (`kinaro`) — the GPUI application: window setup (`main.rs`), global keybindings/actions (`actions.rs`), and all UI (`ui/`).
- **`crates/workspace`** (`ki_workspace`) — the in-memory, live domain model: `Workspace` → `Project` → `TestsContainer`/`ProjectVariables`/`WorkspaceEndpoint`. This is what the UI reads and mutates directly.
- **`crates/project`** (`ki_project`) — the on-disk serialization format only: `ProjectFile` (JSON, `.kpr` extension) and its `File*` types (`FileTestSuite`, `FileTestCase`, `FileTestStep`, `FileEndpoint`, `FileProjectVariables`, ...). Has no notion of GPUI entities or live state.
- **`crates/settings`** (`ki_settings`) — global app settings/state (`GlobalSettings`, `AppState`), persisted as JSON in the OS config dir.
- **`crates/assets`** (`ki_assets`) — embedded assets (fonts, icons, themes) via `rust-embed`, plus GPUI `AssetSource` wiring.
- **`crates/utils`** (`ki_utils`) — small shared helpers and types (e.g. `next_available_name` for generating unique names when duplicating/adding tree nodes, `TestPath` for addressing test tree nodes).

## The File ↔ Workspace split

Every domain concept (project, test suite/case/step, variables, endpoints) exists in **two parallel forms**, and this split is the main thing to keep straight when touching model code:

1. **`ki_project::File*` types** — plain serde structs, the on-disk shape. No behavior beyond (de)serialization.
2. **`ki_workspace` types** (`Project`, `TestSuite`, `TestCase`, `TestStep`, ...) — the live model. These wrap GPUI `Entity<T>`/`Context<T>` where they need to emit change events (e.g. `TestsContainer`, `ProjectVariables`), hold generated-at-runtime data (UUIDs, GPUI-only state), and carry the actual mutation logic.

Conversion is always explicit and one of three methods on the workspace type:
- `from_file(file_thing, ...)` — build the live model from its file representation (ids are read from the file, not regenerated).
- `get_file()` / `to_file()` — produce the serializable form back out.

`Project::save` calls `to_file` and writes JSON via `ki_project::ProjectFile::save`; `Project::load` does the reverse. `Workspace` itself persists a separate small `workspace.json` (list of known project paths + which is active + which tree nodes are expanded) via `ki_settings`'s config dir — this is distinct from each project's own `.kpr` file.

## The test tree (Suite → Case → Step)

This is the most elaborate part of the domain model (`crates/workspace/src/test/`):

- `TestsContainer` owns `Vec<TestSuite>`; a `TestSuite` owns `Vec<TestCase>`; a `TestCase` holds its `meta: TestMetadata` (id, name, description, disabled flag) and a `TestCaseType` (read via `case_type()` / `case_type_mut()`), which is either `CaseMulti { steps: Vec<TestStep> }` (a case with its own steps) or `CaseStep { data }` (a single step promoted directly to case level, so it counts as a case in the suite's list without needing a wrapper). A `CaseStep { data }` is considered a step from the user point of view, but located in a suite.
- Nodes are addressed by **`TestPath`** (defined in `ki_utils`), an enum path (`Suite(suite_id)`, `Case(suite_id, case_id)`, `Step(suite_id, case_id, step_id)`) rather than by index — indices shift as the tree is edited, ids don't. `Case` covers both `CaseMulti` and `CaseStep`: the path doesn't encode the case type, so code that needs it must look up the case itself. Most container methods take a `&TestPath` to say "do this relative to the node at this path".
- `TestPath::is_parent(other)` checks ancestry and is reflexive (a path is its own parent; a suite is a parent of everything under it; a case is a parent of its own steps; a step only of itself). It is how "insert relative to whatever's selected in the UI tree" is implemented: e.g. `TestSuite::add_test_case` finds the case whose id `is_parent` of the given path and inserts right after it, falling back to appending at the end when nothing matches (root suite selected, or an unrelated path).
- Renaming for uniqueness (new cases/steps, duplicates) goes through `ki_utils::next_available_name`, which appends `_1`, `_2`, ... only when the name is already taken.
- `TestsContainer`/`TestSuite`'s mutating methods return `ProjectResult<()>` and emit `TestsContainerEvent` (`TestAdded` / `ConfigChanged`) on success; `Project` subscribes to these and triggers a save + re-emits its own `ProjectEvent`.
- The `data` field in `CaseStep` and `TestStep` is a `StepData` enum (file side: `FileStepData`) with one variant per step kind: `RestStep` (method, endpoint, params and body; see `step_rest.rs`) and `Delay` (milliseconds; see `step_delay.rs`). Each kind is a file struct in `ki_project` and a workspace struct in `ki_workspace`, converted with `From` impls (`From<File*>` for loading, `From<&T>` for saving). New steps and case steps are created as a 1000 ms delay.

The `crates/app` side mirrors this with `TestPath`-driven `ProjectTreeDelegate` methods (`crates/app/src/ui/side_bar/project_tree_tab.rs`) that resolve a selected tree row's index to its `TestPath` before calling into `ki_workspace`.

## GPUI/gpui-kit conventions

- Stateful model types are wrapped in `Entity<T>` and mutated via `entity.update(cx, |state, cx| ...)`; reads go through `entity.read(cx)`.
- Cross-entity reactions use `EventEmitter<SomeEvent>` + `cx.emit(...)` on the producer and `cx.subscribe(&entity, Self::on_event)` on the consumer (see `Workspace` ↔ `Project` ↔ `TestsContainer`/`ProjectVariables` for the chain of subscriptions that ends in persisting to disk).
- UI actions are declared with the `actions!([...])` macro and bound to keys in `actions::init` (`crates/app/src/actions.rs`); a context key (e.g. `PROJECT_TREE_CONTEXT_KEY`) scopes a binding to a specific focused view.
- The project tree view implements the app-local `TreeDelegate` trait (`crates/app/src/ui/components/tree.rs`) to drive a generic `TreeState<D>` list component.

## Project conventions
- Function documentation should include an Error section whenever it returns a Result, or a derived type (ProjectResult, WorkspaceResult...).
- Function documentation should include an Event section whenever it emits an event (through the use of `Context::emit(event)`)
- Documentation of functions/fields should avoid referencing more-private functions and fields