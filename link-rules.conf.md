# Link Rules Configuration File Format

This document describes the format of the `link-rules.conf` file used by pipewire-api to automatically create and manage audio links.

## File Locations

Link rules can be configured in two locations (in order of precedence):

1. **System-wide**: `/etc/pipewire-api/link-rules.conf`
2. **User-specific**: `~/.config/pipewire-api/link-rules.conf`

Rules from both files are loaded and applied. User rules are loaded after system rules.

## File Format

The configuration file uses JSON format and contains an array of link rule objects.

### Basic Structure

```json
[
  {
    "name": "My Link Rule",
    "source": {
      "node.name": "^source_node_pattern$"
    },
    "destination": {
      "object.path": "destination:pattern"
    },
    "type": "link",
    "link_at_startup": true,
    "relink_every": 10
  }
]
```

## Link Rule Properties

### Required Properties

- **`name`** (string): Descriptive name for the link rule. This name is used as the `object.name` property for created links in PipeWire.

- **`type`** (string): Type of operation. Currently supported:
  - `"link"`: Create a link between nodes

- **`source`** (object): Identifies the source node. Must contain at least one of:
  - `"node.name"`: Regex pattern to match node name
  - `"node.nick"`: Regex pattern to match node nickname
  - `"object.path"`: Regex pattern to match object path

- **`destination`** (object): Identifies the destination node. Same structure as `source`.

### Optional Properties

- **`link_at_startup`** (boolean, default: `true`): Whether to apply this rule when the server starts.

- **`relink_every`** (number, default: `0`): How often to check and recreate links (in seconds).
  - `0`: Create link only once. If someone removes it, it won't be recreated.
  - `>0`: Check every N seconds and recreate the link if it doesn't exist.

- **`info_level`** (string, default: `"info"`): Log level for normal operations (link created, already exists, removed). Options:
  - `"debug"`: Only show at debug level
  - `"info"`: Informational messages (default)
  - `"warn"`: Show as warnings
  - `"error"`: Show as errors

- **`error_level`** (string, default: `"error"`): Log level for error conditions (node not found, can't create link). Options:
  - `"debug"`: Only show at debug level
  - `"info"`: Informational messages
  - `"warn"`: Show as warnings
  - `"error"`: Show as errors (default)
  
  These log levels allow fine-grained control. For example, optional links can use `info_level: "debug"` to avoid cluttering logs, while critical links can use `error_level: "error"` to ensure failures are visible.

## Pattern Matching

All node identifiers use **regular expressions** for matching. Common patterns:

- **Exact match**: `"^node_name$"`
- **Starts with**: `"^prefix"`
- **Contains**: `"substring"`
- **Wildcard**: `".*"` (matches any characters)
- **Character class**: `"node[12]"` (matches node1 or node2)

### Special Characters in Regex

Remember to escape special regex characters with backslashes:
- `.` becomes `\\.` (to match a literal dot)
- `*` becomes `\\*` (to match a literal asterisk)

## Examples

### Example 1: Simple Link

Link a specific output node to a specific input node:

```json
[
  {
    "name": "MyApp to Speakers",
    "source": {
      "node.name": "^myapp\\.output$"
    },
    "destination": {
      "node.name": "^alsa_output\\.speakers$"
    },
    "type": "link",
    "link_at_startup": true,
    "relink_every": 0
  }
]
```

### Example 2: Pattern Matching with Monitoring

Link any SpeakerEQ output to HiFiBerry playback, and check every 10 seconds:

```json
[
  {
    "name": "SpeakerEQ to HiFiBerry",
    "source": {
      "node.name": "^speakereq.x.\\.output$"
    },
    "destination": {
      "object.path": "alsa:.*:sndrpihifiberry:.*:playback"
    },
    "type": "link",
    "link_at_startup": true,
    "relink_every": 10,
    "info_level": "debug",
    "error_level": "error"
  }
]
```

### Example 3: Multiple Rules

```json
[
  {
    "name": "Browser to Default",
    "source": {
      "node.name": "^firefox.*"
    },
    "destination": {
      "node.name": "^alsa_output\\.default$"
    },
    "type": "link",
    "link_at_startup": true,
    "relink_every": 5
  },
  {
    "name": "Music Player to DAC",
    "source": {
      "node.name": "^mpd\\.output$"
    },
    "destination": {
      "object.path": "alsa:.*:usb-dac:.*:playback"
    },
    "type": "link",
    "link_at_startup": true,
    "relink_every": 0
  }
]
```

## Testing Configuration

To test your configuration file:

1. Create the configuration file in the appropriate location
2. Run: `pipewire-api` (with logging): `RUST_LOG=info pipewire-api`
3. Check the logs for messages like:
   - `Loaded N rule(s) from system config`
   - `Loaded N rule(s) from user config`
   - `Startup rule 0 applied: X/Y links successful`

## Troubleshooting

### Rules not loading

- Check JSON syntax with `jq . < link-rules.conf`
- Verify file permissions are readable
- Check logs with `RUST_LOG=debug pipewire-api`

### Rules not matching nodes

- Use `pw-dump | jq '.[] | select(.type == "PipeWire:Interface:Node")'` to see available nodes
- Check node properties: `node.name`, `node.nick`, `object.path`
- Test regex patterns at https://regex101.com/

### Links not persisting

- Verify `object.linger` is working (should be automatic)
- Check if links appear with: `link-nodes -l` or use PipeWire's native `pw-link -l`
- Inspect link properties: `pw-dump | jq '.[] | select(.type == "PipeWire:Interface:Link")'`

## See Also

- `pipewire-api(1)` - Main API server documentation
- `link-nodes(1)` - PipeWire link management tool
- PipeWire documentation: https://docs.pipewire.org/
