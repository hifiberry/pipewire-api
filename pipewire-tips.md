# PipeWire Tips and Tricks

This document contains useful information and discoveries about working with PipeWire that may not be immediately obvious from the documentation.

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
