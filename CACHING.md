# Internal Caching

The pipewire-api server uses several caching layers to avoid repeated
subprocess calls to pw-cli, wpctl, and pw-link. PipeWire object IDs are
stable during a session, so cached lookups are safe.

## Cache layers

### 1. AppState object cache (`api_server.rs`)

- **Storage:** `Arc<RwLock<Vec<PwObject>>>` in `AppState`
- **Populated:** once at server startup (`pipewire-api.rs:58`)
- **Refreshed:** manually via `POST /api/v1/cache/refresh`
- **No TTL** -- stays until explicitly refreshed
- **Used by:**
  - `listing.rs` -- `get_object_by_id()` checks cache, falls back to pwcli
  - `properties.rs` -- `list_all_properties()` and `get_object_properties()`
    check cache, fall back to pwcli
- **Not used by:**
  - `listing.rs` `list_all()` -- always calls pwcli (returns full list)
  - `graph.rs` -- always calls pwcli (needs ports, links, clients too)
  - `link_manager_cli.rs` -- always calls pwcli (needs real-time link state)
- **Lookup:** O(n) linear scan of Vec (no indexing by ID)

### 2. pwcli object cache (`pwcli.rs`)

- **Storage:** `static OBJECT_CACHE: OnceLock<Mutex<HashMap<u32, PwObject>>>`
- **Populated:** lazily on first `get_object()` cache miss
- **Refreshed:** automatically on cache miss (calls `list_all()` then
  populates entire HashMap)
- **No TTL** -- once an ID is cached it stays cached; a miss triggers
  a full refresh
- **Used by:** `get_object(id)`, and transitively by `find_node_by_name()`
  and `find_node_by_match()`
- **Lookup:** O(1) HashMap by object ID

### 3. pwcli node name cache (`pwcli.rs`)

- **Storage:** `static NODE_CACHE: OnceLock<Mutex<HashMap<String, u32>>>`
  (maps `node.name` -> object ID)
- **Populated:** lazily on first `find_node_by_name()` or
  `find_node_by_match()` call
- **Refreshed:** automatically on cache miss (calls `list_nodes()`)
- **No TTL**
- **Used by:** `find_node_by_name()`, `find_node_by_match()`,
  `find_name_by_id()`
- **Lookup:** O(1) HashMap by node name

### 4. NodeState parameter cache (`api_server.rs`)

- **Storage:** `Arc<Mutex<Option<HashMap<String, ParameterValue>>>>` per
  node (one for speakereq, one for riaa)
- **Populated:** lazily on first `get_params()` call (runs
  `pw-cli enum-params`)
- **Invalidated:** after every `set_parameters()` call (cache set to `None`)
- **Refreshed:** via `POST /api/v1/module/speakereq/refresh` or
  `POST /api/v1/module/riaa/refresh`
- **No TTL** -- stays cached until a write or explicit refresh
- **Used by:** speakereq and riaa parameter get/set handlers

## Call flow for object lookups

```
Handler needs object by ID
  |
  +--> AppState.get_object_by_id()     [O(n) Vec scan]
  |      |
  |      +-- hit  --> return cached object
  |      +-- miss --> fall through
  |
  +--> pwcli::get_object(id)           [O(1) HashMap]
         |
         +-- hit  --> return cached object
         +-- miss --> pwcli::refresh_object_cache()
                        |
                        +--> pw-cli ls  (subprocess)
                        +--> populate HashMap
                        +--> return object or None
```

## What is NOT cached

- **`pwcli::list_all()`** -- always runs `pw-cli ls` subprocess.
  Used when the full current object list is needed (graph generation,
  listing all objects, link manager rule evaluation).
- **wpctl calls** -- `list_volumes()`, `get_volume()`, `set_volume()`,
  `get_default_sink()`, `get_default_source()` always run wpctl.
  Volumes change frequently so caching would require short TTL.
- **pwlink calls** -- `list_links()`, `list_output_ports()`,
  `list_input_ports()`, `create_link()`, `remove_link()`, `find_link()`
  always run pw-link. Link state changes during operations.
- **link_manager_cli `LinkData::load()`** -- always calls `list_all()`
  because it needs current link state for rule evaluation. Runs every
  1+ seconds via the link scheduler.

## Design rationale

PipeWire object IDs and node names are stable during a session (they
don't change until pipewire restarts). This makes them safe to cache
indefinitely. Volumes and link state, on the other hand, change
frequently and are always fetched fresh.

The two-level object cache (AppState Vec + pwcli HashMap) is historical.
AppState was added first for API handlers; the pwcli HashMap was added
later to speed up internal lookups from `find_node_by_name()` and
`find_node_by_match()` which previously called `list_all()` for every
single ID lookup.
