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

### Wildcard Matching

All identifier fields support wildcard matching using `*`:

- `"alsa_*"` matches "alsa_output.usb", "alsa_input.usb", etc.
- `"*speaker*"` matches any node with "speaker" in the name
- `"*.usb"` matches names ending with ".usb"

Wildcards are converted to regex patterns, so `*` matches any sequence of characters.

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
      "node_name": "alsa_output.*"
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

## Current Limitations

**Note:** The actual link creation/destruction functionality is not yet fully implemented. The current implementation:

1. ✅ Can list all existing PipeWire links
2. ✅ Can find and match nodes using wildcards
3. ✅ Can validate link rules
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
