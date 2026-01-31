# Link Management API

The PipeWire API now includes endpoints for managing links between PipeWire nodes using rule-based matching.

## Endpoints

### GET /api/v1/links

List all active PipeWire links.

**Response:**
```json
[
  {
    "id": 123,
    "output_node_id": 45,
    "output_port_id": 67,
    "input_node_id": 89,
    "input_port_id": 90,
    "output_node_name": "alsa_output.usb",
    "input_node_name": "alsa_input.usb"
  }
]
```

### POST /api/v1/links/apply

Apply a single link rule.

**Request Body:**
```json
{
  "source": {
    "node_name": "alsa_output.*",
    "node_nick": null,
    "object_path": null
  },
  "destination": {
    "node_name": "speakereq2x2",
    "node_nick": null,
    "object_path": null
  },
  "link_type": "link"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Link rule applied successfully"
}
```

### GET /api/v1/links/default

Get the default link rules configured for the system.

**Response:**
```json
[
  {
    "source": {
      "node_name": "^speakereq.x.\\.output$",
      "node_nick": null,
      "object_path": null
    },
    "destination": {
      "node_name": null,
      "node_nick": null,
      "object_path": "alsa.*sndrpihifiberry.*playback"
    },
    "link_type": "link"
  }
]
```

### POST /api/v1/links/apply-defaults

Apply all default link rules.

**Response:**
```json
{
  "total": 1,
  "successful": 1,
  "failed": 0,
  "results": [
    {
      "success": true,
      "message": "Default rule 1 applied successfully"
    }
  ]
}
```

### POST /api/v1/links/batch

Apply multiple link rules in sequence.

**Request Body:**
```json
{
  "rules": [
    {
      "source": {
        "node_name": "alsa_output.*"
      },
      "destination": {
        "node_name": "speaker*"
      },
      "link_type": "link"
    },
    {
      "source": {
        "object_path": "/org/freedesktop/pipewire/node/45"
      },
      "destination": {
        "node_nick": "Output Device"
      },
      "link_type": "unlink"
    }
  ]
}
```

**Response:**
```json
{
  "total": 2,
  "successful": 1,
  "failed": 1,
  "results": [
    {
      "success": true,
      "message": "Rule 1 applied successfully"
    },
    {
      "success": false,
      "message": "Rule 2 failed: Link creation not yet implemented"
    }
  ]
}
```

## Link Rules

A link rule consists of:

- **source**: Node identifier for the source (output) node
- **destination**: Node identifier for the destination (input) node
- **link_type**: Either "link" (create connection) or "unlink" (remove connection)

## Node Identifiers

Nodes can be identified by any of these properties (only one is required):

- **node_name**: The `node.name` property (e.g., "alsa_output.usb-0")
- **node_nick**: The `node.nick` property (e.g., "USB Audio")
- **object_path**: The `object.path` property (e.g., "/org/freedesktop/pipewire/node/45")

### Regex Matching

All identifier fields support regular expression matching:

- `"^alsa_.*"` matches names starting with "alsa_"
- `".*speaker.*"` matches any node with "speaker" anywhere in the name
- `".*\\.usb$"` matches names ending with ".usb"
- `"^speakereq.x.\\.output$"` matches "speakereq2x2.output", "speakereq4x4.output", etc.

Common regex patterns:
- `.` matches any single character
- `.*` matches any sequence of characters (including empty)
- `.+` matches one or more characters
- `^` matches the start of the string
- `$` matches the end of the string
- `\\.` matches a literal dot (escape with backslash)

## Examples

### Example 1: Link a specific output to a specific input

```bash
curl -X POST http://localhost:2716/api/v1/links/apply \
  -H "Content-Type: application/json" \
  -d '{
    "source": {
      "node_name": "alsa_output.pci-0000_00_1f.3.analog-stereo"
    },
    "destination": {
      "node_name": "speakereq2x2"
    },
    "link_type": "link"
  }'
```

### Example 2: Link all ALSA outputs to a device

```bash
curl -X POST http://localhost:2716/api/v1/links/apply \
  -H "Content-Type: application/json" \
  -d '{
    "source": {
      "node_name": "^alsa_output\\..*"
    },
    "destination": {
      "node_name": "my_audio_device"
    },
    "link_type": "link"
  }'
```

### Example 3: Unlink nodes by object path

```bash
curl -X POST http://localhost:2716/api/v1/links/apply \
  -H "Content-Type: application/json" \
  -d '{
    "source": {
      "object_path": "/org/freedesktop/pipewire/node/45"
    },
    "destination": {
      "object_path": "/org/freedesktop/pipewire/node/67"
    },
    "link_type": "unlink"
  }'
```

### Example 4: List all links

```bash
curl http://localhost:2716/api/v1/links
```

### Example 5: Get default link rules

```bash
curl http://localhost:2716/api/v1/links/default
```

### Example 6: Apply default link rules

```bash
curl -X POST http://localhost:2716/api/v1/links/apply-defaults
```

This will automatically link speakereq output nodes to ALSA HiFiBerry playback devices.

### Example 7: Get link rules status

```bash
curl http://localhost:2716/api/v1/links/status
```

**Response:**
```json
{
  "rules": [
    {
      "index": 0,
      "rule": {
        "name": "SpeakerEQ to HiFiBerry",
        "source": {
          "node.name": "^speakereq.x.\\.output$",
          "node.nick": null,
          "object.path": null
        },
        "destination": {
          "node.name": null,
          "node.nick": null,
          "object.path": "alsa.*sndrpihifiberry.*playback"
        },
        "type": "link",
        "link_at_startup": true,
        "relink_every": 5
      },
      "status": {
        "last_run": "2026-01-31T14:32:15.123456Z",
        "last_run_timestamp": 1738330335,
        "links_created": 2,
        "links_failed": 0,
        "last_error": null,
        "total_runs": 42
      }
    }
  ]
}
```

This endpoint shows all configured link rules along with their execution status, including:
- `last_run`: ISO 8601 timestamp of the last execution
- `last_run_timestamp`: Unix timestamp of the last execution
- `links_created`: Number of links successfully created on the last run
- `links_failed`: Number of links that failed on the last run
- `last_error`: Last error message, if any
- `total_runs`: Total number of times this rule has been executed

## Default Link Rules

The system comes with pre-configured default link rules that can be retrieved via `/api/v1/links/default` or applied via `/api/v1/links/apply-defaults`.

**Default Rule 1: SpeakerEQ to HiFiBerry**
- **Source**: `node.name` matching `"^speakereq.x.\\.output$"` (matches speakereq2x2.output, speakereq4x4.output, etc.)
- **Destination**: `object.path` matching `"alsa.*sndrpihifiberry.*playback"`
- **Action**: Link (create connection)

This rule automatically routes the output of SpeakerEQ nodes to HiFiBerry ALSA playback devices.

## Automatic Link Management

The API server includes an automatic link scheduler that monitors and applies link rules based on the configuration file (`/etc/pipewire-api/link-rules.conf` or `~/.config/pipewire-api/link-rules.conf`). 

Rules can be configured to:
- Run at startup (`link_at_startup: true`)
- Re-link periodically (`relink_every: N` seconds, where 0 means link only once)

The link scheduler tracks the status of each rule and can be queried via the `/api/v1/links/status` endpoint to monitor when rules last ran and their success/failure status.
4. ❌ Cannot actually create new links (returns error indicating this needs implementation)
5. ⚠️ Unlink operation is not yet fully implemented

The link creation requires proper integration with the PipeWire Core API's `create_object` method with appropriate factory parameters. This is a non-trivial operation that needs further development.

## Future Enhancements

- Implement actual link creation using PipeWire Core API
- Implement unlink operation to destroy existing links
- Add port-level linking (currently only supports node-level)
- Add link state monitoring
- Add link property management
- Add validation for port compatibility
