# PipeWire Tips and Tricks

This document contains useful information and discoveries about working with PipeWire that may not be immediately obvious from the documentation.

## Setting Custom Node Parameters

### Using pw-cli

pw-cli can set custom node parameters (like speakereq2x2 parameters) using the Props param type with a special array format:

```bash
# Single parameter
pw-cli set-param <node-id> Props '{ "params": ["param-name", value] }'

# Multiple parameters in one call (recommended for batching)
pw-cli set-param <node-id> Props '{ "params": ["param1", value1, "param2", value2, ...] }'
```

Example setting speakereq2x2 EQ parameters:
```bash
pw-cli set-param 44 Props '{ "params": [
  "speakereq2x2:output_0_eq_1_type", 3,
  "speakereq2x2:output_0_eq_1_f", 1000.0,
  "speakereq2x2:output_0_eq_1_q", 2.5,
  "speakereq2x2:output_0_eq_1_gain", 3.0,
  "speakereq2x2:output_0_eq_1_enabled", true
] }'
```

The parameters are stored in a Struct inside the "params" property (key 524289). Reading them:
```bash
pw-cli enum-params <node-id> Props
```

**Important:** The params value must be an array alternating between parameter name strings and their values, NOT a JSON object. This is because it maps to a SPA Struct in PipeWire's type system.

## Link Creation and Persistence

### The `object.linger` Property

When creating links programmatically using `core.create_object::<pw::link::Link>()`, links will be automatically destroyed when the client that created them disconnects from PipeWire. This is the default behavior to prevent orphaned links.

**Problem:** Links created by a short-lived client (like a CLI tool or API call) disappear immediately after the program exits.

**Solution:** Add the `object.linger` property when creating the link:

```rust
let proxy = core.create_object::<pw::link::Link>(
    "link-factory",
    &pw::properties::properties! {
        "link.output.port" => output_port_id.to_string(),
        "link.input.port" => input_port_id.to_string(),
        "object.linger" => "true",  // Makes the link persist
    },
)?;
```

With `object.linger` set to `"true"`, the link will persist in the PipeWire graph even after the client disconnects.

### Other Link Properties Tested

- **`link.passive`**: This property alone is NOT sufficient to make links persist. Links created with only `link.passive = true` will still be destroyed when the client disconnects.

- **Combination not needed**: While some examples show both `link.passive` and `object.linger`, testing shows that `object.linger` alone is sufficient for link persistence.

### Mainloop Processing

After creating links with `core.create_object()`, it's recommended to run the mainloop briefly (100-500ms) to allow PipeWire to process the link creation before the client disconnects:

```rust
let process_mainloop = mainloop.clone();
let _timer = mainloop.loop_().add_timer(move |_| {
    process_mainloop.quit();
});
_timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
mainloop.run();
```

This ensures the server has time to fully process and register the links before the client exits.

## Related Resources

- PipeWire link creation example: [create-delete-remote-objects.rs](https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/blob/master/pipewire/examples/create-delete-remote-objects.rs)
- PipeWire properties documentation: [Properties](https://docs.pipewire.org/group__pw__properties.html)
