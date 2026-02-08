# PipeWire API - Improvement Proposals

Analysis of the full codebase (~4000 lines across 25 Rust source files).
Findings ordered by severity: bugs first, then architecture, then code quality.

---

## 1. BUG: Link Rule Endpoints Not Mounted

**Files:** `src/links.rs`, `src/pipewire-api.rs`

The `links.rs` module defines a router with rule-based link management endpoints
(`/api/v1/links/apply`, `/api/v1/links/batch`, `/api/v1/links/default`,
`/api/v1/links/apply-defaults`, `/api/v1/links/status`) but this router is never
merged in `pipewire-api.rs`. The endpoint list in `api/mod.rs` documents these
endpoints, the README documents them, but they are unreachable.

**Fix:** Add `.merge(pw_api::links::create_router(app_state.clone()))` to the
router chain in `pipewire-api.rs`. Note: the `/api/v1/links` GET route in
`links.rs` conflicts with the one in `api/links.rs` -- one should be removed
or they should be reconciled.

---

## 2. BUG: Blocking Subprocess Calls in Async Handlers

**Files:** `src/api/listing.rs`, `src/api/volume.rs`, `src/api/links.rs`,
`src/api/properties.rs`, `src/api_server.rs`

Most API handlers call `std::process::Command` (via `pwcli`, `wpctl`, `pwlink`)
directly from async context without `tokio::task::spawn_blocking`. This blocks
the tokio runtime thread and degrades throughput under concurrent requests.

Only `src/links.rs` (the top-level module) correctly uses `spawn_blocking`.

**Fix:** Wrap all subprocess calls in `spawn_blocking`. Consider adding async
wrapper functions to `pwcli`, `wpctl`, and `pwlink` modules.

---

## 3. Volume Clamp Inconsistency

**Files:** `src/wpctl.rs:181`, `src/config.rs:22`

`wpctl::set_volume()` clamps volume to `0.0..1.5`, but the `VolumeRule` struct
documents the range as `0.0 - 2.0, where 1.0 = 100%`. These should agree.

**Fix:** Decide on the maximum volume and apply it consistently.

---

## 4. Duplicated `regex_match` Function

**Files:** `src/linker.rs:103`, `src/link_manager_cli.rs:116`

Identical function in two modules.

**Fix:** Move to a shared utility module (e.g., `src/util.rs`) or keep only the
one in `linker.rs` and import it in `link_manager_cli.rs`.

---

## 5. Duplicated Node Matching Logic

**Files:** `src/linker.rs:46` (`NodeIdentifier::matches_properties`),
`src/link_manager_cli.rs:125` (`matches_identifier`)

Both do regex-based matching against node properties. The `link_manager_cli`
version works on a custom `NodeInfo` struct while `linker` works on a `HashMap`.

**Fix:** Consolidate into `NodeIdentifier::matches_properties()` and have
`link_manager_cli` convert its data to use that method. Or add a
`NodeIdentifier::matches_node_info()` that delegates to the shared regex logic.

---

## 6. Duplicated Enums and Structs

**Files:** `src/link_manager_cli.rs`, `src/pwlink.rs`, `src/api/links.rs`,
`src/links.rs`

- `PortDirection` enum exists in both `link_manager_cli.rs` and `pwlink.rs`
- `LinkInfo` struct exists in `link_manager_cli.rs`, `api/links.rs`, and
  `links.rs`, each with slightly different fields

**Fix:** Define canonical types in one place and re-use. `pwlink::PortDirection`
should be the single source, and there should be one `LinkInfo` type (possibly
with optional fields) used across the codebase.

---

## 7. Highly Repetitive Config Loading Functions

**File:** `src/config.rs`

`load_all_link_rules()`, `load_all_param_rules()`, and `load_all_volume_rules()`
are nearly identical -- same user-then-system-config pattern, same logging, same
error handling. Only the file path and type differ.

**Fix:** Extract a generic `load_all_rules<T: DeserializeOwned>(user_path, system_path) -> Vec<T>`
function that the three loaders delegate to.

---

## 8. Inefficient `get_object(id)` Implementation

**File:** `src/pwcli.rs:288`

`get_object(id)` calls `list_all()` (which runs `pw-cli ls` and parses the
full output) just to find one object by ID. This is called from
`find_node_by_name` (potentially twice if cache misses), making a cache-miss
lookup run `pw-cli ls` 3+ times.

**Fix:** Either always use the AppState cache for lookups, or implement a
targeted `pw-cli info <id>` call.

---

## 9. `speakereq.rs` Exceeds 1000-Line Limit

**File:** `src/speakereq.rs` (1075 lines)

The project's own AI_INSTRUCTIONS.md says files should not exceed 1000 lines.

**Fix:** Split into submodules, e.g.:
- `speakereq/mod.rs` - router creation, shared helpers
- `speakereq/eq.rs` - EQ band get/set/clear handlers
- `speakereq/crossbar.rs` - crossbar matrix handlers
- `speakereq/status.rs` - status/structure/config/capabilities

---

## 10. Hardcoded 2x2 Assumption in SpeakerEQ

**File:** `src/speakereq.rs`

`get_status()` (line 681) hardcodes 2 inputs and 2 outputs, and the crossbar
matrix struct has fixed fields (`input_0_to_output_0`, etc.). Meanwhile,
`get_config()` dynamically probes input/output counts. The crossbar matrix
should also be dynamic (use a `Vec<Vec<f32>>` as `get_crossbar` already does).

**Fix:** Make `get_status()` use the same dynamic probing as `get_config()`.
Consider replacing `CrossbarMatrix` with the dynamic `CrossbarMatrixResponse`.

---

## 11. Duplicated Crossbar Parameter Reading

**File:** `src/speakereq.rs`

The crossbar parameter reading code (4 `params.get()` calls with the same
match-and-unwrap pattern) is duplicated between `get_status()` (lines 552-582)
and `get_crossbar()` (lines 708-738).

**Fix:** Extract a `read_crossbar_matrix(params, prefix) -> CrossbarMatrix`
helper.

---

## 12. Repetitive Parameter Extraction Pattern

**Files:** `src/speakereq.rs`, `src/riaa.rs`

The pattern `params.get(key).and_then(|v| match v { Type(x) => Some(*x), _ => None }).unwrap_or(default)`
is repeated dozens of times.

**Fix:** Add helper methods to `ParameterValue`:
```rust
impl ParameterValue {
    fn as_float(&self) -> Option<f32>;
    fn as_int(&self) -> Option<i32>;
    fn as_bool(&self) -> Option<bool>;
}
```
Then the pattern becomes `params.get(key).and_then(|v| v.as_float()).unwrap_or(0.0)`.

---

## 13. `ParameterValue::to_string()` Shadows `ToString` Trait

**File:** `src/parameters.rs:27`

The method `fn to_string(&self) -> String` shadows the standard `ToString`
trait. This is a Clippy warning (`inherent_to_string`) and can cause confusion.

**Fix:** Implement `std::fmt::Display` for `ParameterValue` instead, which
auto-provides `ToString`.

---

## 14. No CORS Middleware

**File:** `src/pipewire-api.rs`

`tower-http` is a dependency with the `cors` feature enabled in Cargo.toml, but
no CORS middleware is applied. If the API is accessed from a web UI (likely for
HiFiBerry), browsers will block cross-origin requests.

**Fix:** Add CORS layer:
```rust
use tower_http::cors::{CorsLayer, Any};
let app = app.layer(CorsLayer::permissive());
```

---

## 15. Mutex/RwLock Poison Risk

**File:** `src/api_server.rs`

All lock accesses use `.unwrap()` (e.g., `self.object_cache.read().unwrap()`).
If any thread panics while holding the lock, the lock becomes poisoned and all
subsequent `.unwrap()` calls will panic, crashing the server.

**Fix:** Either handle the poison error (`.unwrap_or_else(|e| e.into_inner())`)
or switch to `parking_lot` mutexes which don't poison.

---

## 16. Regex Compiled on Every Call

**Files:** `src/linker.rs:103`, `src/link_manager_cli.rs:116`,
`src/param_rules.rs:67`

`Regex::new()` is called every time a pattern needs to be matched. Regex
compilation is expensive.

**Fix:** Use `once_cell::sync::Lazy` with a `HashMap<String, Regex>` cache, or
pre-compile patterns when loading rules and store the compiled `Regex` alongside
the pattern string.

---

## 17. No Input Validation on Block/Band Path Parameters

**File:** `src/speakereq.rs`

The `block` and `band` path parameters in EQ endpoints (e.g.,
`/api/module/speakereq/eq/:block/:band`) are passed directly into parameter key
construction without validation. While not a security issue (they become pw-cli
parameter names), invalid values just return "not found" without a helpful
message.

**Fix:** Validate `block` against known block IDs and `band` against the slot
count before constructing parameter keys.

---

## 18. `param_rules.rs` Duplicates `set_params_via_pwcli` Logic

**Files:** `src/param_rules.rs:168-188`, `src/api_server.rs:357-396`

The JSON construction for `pw-cli set-param` is duplicated between
`param_rules::apply_param_rules` and `NodeState::set_params_via_pwcli`.

**Fix:** Have `param_rules` use `NodeState::set_params_via_pwcli()` or extract
the JSON-building logic into a shared function.

---

## 19. Section Tracking Duplicated in wpctl

**File:** `src/wpctl.rs`

The section-tracking logic (Sinks, Sources, Filters, Devices, Streams) is
duplicated between `parse_wpctl_status()` (line 49) and `get_object_info()`
(line 155).

**Fix:** Extract section tracking into a helper or parse wpctl status once and
cache the result.

---

## 20. Missing Graceful Shutdown

**File:** `src/pipewire-api.rs`

The server doesn't handle `SIGTERM`/`SIGINT` for graceful shutdown. The
auto-save task and link scheduler will be abruptly terminated.

**Fix:** Use `tokio::signal` to listen for shutdown signals and pass a
cancellation token to background tasks. `axum::serve` supports
`.with_graceful_shutdown()`.

---

## Summary by Priority

| Priority | Items | Impact |
|----------|-------|--------|
| **Bug** | #1 (unmounted router), #3 (volume clamp) | Features broken/inconsistent |
| **Performance** | #2 (blocking in async), #8 (redundant pw-cli calls), #16 (regex recompilation) | Server throughput and latency |
| **Maintainability** | #4-7, #11-12, #18-19 (code duplication) | Harder to evolve, bug risk |
| **Architecture** | #9 (file size), #10 (hardcoded 2x2), #14 (CORS), #15 (lock poisoning), #20 (shutdown) | Robustness and extensibility |
| **Code quality** | #13 (trait shadow), #17 (validation) | Clippy warnings, UX |
