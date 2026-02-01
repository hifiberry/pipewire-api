# PipeWire REST API Specification

## Overview
REST API for controlling PipeWire audio system, with specialized support for the SpeakerEQ 2x2 audio filter plugin.
Includes endpoints for listing PipeWire objects, inspecting properties, managing links, and controlling SpeakerEQ parameters.

## Base URL
`http://localhost:2716/api/v1`

Note: The server binds to all interfaces (0.0.0.0) by default. Use `--localhost` flag to restrict to localhost only.

## Endpoints

### Generic PipeWire Endpoints

#### List All Objects
```
GET /ls
```
Returns all PipeWire objects (nodes, devices, ports, links, clients, modules, factories).

**Response:**
```json
{
  "objects": [
    {
      "id": 45,
      "name": "speakereq2x2",
      "type": "node"
    },
    {
      "id": 67,
      "name": "HiFiBerry DAC",
      "type": "device"
    }
  ]
}
```

#### List Nodes
```
GET /ls/nodes
```
Returns all PipeWire nodes (audio/video processing elements).

#### List Devices
```
GET /ls/devices
```
Returns all PipeWire devices (audio/video hardware).

#### List Ports
```
GET /ls/ports
```
Returns all PipeWire ports (connection endpoints).

#### List Links
```
GET /ls/links
```
Returns all PipeWire links (active connections).

#### List Clients
```
GET /ls/clients
```
Returns all PipeWire clients (applications).

#### List Modules
```
GET /ls/modules
```
Returns all PipeWire modules.

#### List Factories
```
GET /ls/factories
```
Returns all PipeWire object factories.

#### Get All Properties
```
GET /properties
```
Returns all objects with their complete property dictionaries.

**Response:**
```json
{
  "objects": [
    {
      "id": 45,
      "name": "speakereq2x2",
      "type": "node",
      "properties": {
        "node.name": "speakereq2x2",
        "node.description": "SpeakerEQ 2x2",
        "media.class": "Audio/Filter"
      }
    }
  ]
}
```

#### Get Object Properties by ID
```
GET /properties/:id
```
Returns properties for a specific object.

**Parameters:**
- `id` (path): Object ID

**Response:**
```json
{
  "id": 45,
  "name": "speakereq2x2",
  "type": "node",
  "properties": {
    "node.name": "speakereq2x2",
    "node.description": "SpeakerEQ 2x2",
    "media.class": "Audio/Filter"
  }
}
```

### Device Management Endpoints

#### List All Devices
```
GET /api/v1/devices
```
Returns all PipeWire devices with their basic information and properties.

**Response:**
```json
[
  {
    "id": 56,
    "name": "alsa_card.pci-0000_00_1f.3",
    "properties": {
      "device.name": "alsa_card.pci-0000_00_1f.3",
      "device.description": "Built-in Audio",
      "api.alsa.card": "0"
    }
  }
]
```

### Volume Management Endpoints

The volume API provides unified volume control for both hardware devices and software sinks (audio outputs). The API automatically detects the object type and uses the appropriate PipeWire parameters.

#### List All Volumes
```
GET /api/v1/volume
```
Returns all devices and sinks that have volume control with their current volume levels. Only objects that actually have a volume setting are included in the response.

**Response:**
```json
[
  {
    "id": 56,
    "name": "alsa_card.pci-0000_00_1f.3",
    "object_type": "device",
    "volume": 1.0
  },
  {
    "id": 81,
    "name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
    "object_type": "sink",
    "volume": 0.85
  }
]
```

**Field Descriptions:**
- `id`: Unique PipeWire object ID
- `name`: Object name (device.name or node.name)
- `object_type`: Either `"device"` (hardware device) or `"sink"` (software audio output)
- `volume`: Float value where 1.0 = 100%, 0.5 = 50%, etc. (only present if volume is available)

**Notes:**
- Only objects with an actual volume setting are returned
- Objects without volume control are automatically filtered out

#### Get Volume by ID
```
GET /api/v1/volume/:id
```
Returns volume information for a specific device or sink. The API automatically detects whether the ID refers to a device or sink.

**Parameters:**
- `id` (path): Object ID (device or node ID)

**Response:**
```json
{
  "id": 81,
  "name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
  "object_type": "sink",
  "volume": 0.85
}
```

**Error Response:**
Returns 404 if the object is not found or does not have volume control.

#### Set Volume by ID
```
PUT /api/v1/volume/:id
```
Sets the volume for a specific device or sink. Works for both hardware devices and software sinks.

**Parameters:**
- `id` (path): Object ID
- `volume` (body): Float value (0.0 to 2.0 typically, where 1.0 = 100%)

**Request:**
```json
{
  "volume": 0.75
}
```

**Response:**
```json
{
  "volume": 0.75
}
```

**Implementation Notes:**
- For devices: Uses Route parameters with channelVolumes
- For sinks: Uses Props parameters with volume property
- Volume range typically 0.0-2.0 (0-200%), but values above 1.0 may cause clipping

#### Save All Volumes
```
POST /api/v1/volume/save
```
Saves the current volumes of all devices and sinks to the state file (`~/.state/pipewire-api/volume.state`).

**Response:**
```json
{
  "success": true,
  "message": "Volume state saved"
}
```

#### Save Specific Volume
```
POST /api/v1/volume/save/:id
```
Saves the current volume of a specific device or sink to the state file.

**Parameters:**
- `id` (path): Object ID

**Response:**
```json
{
  "success": true,
  "id": 56,
  "volume": 0.85,
  "message": "Volume state saved"
}
```

**Volume State File:**
- Saved volumes persist across restarts when `use_state_file: true` is set in volume.conf
- State file location: `~/.state/pipewire-api/volume.state`
- State file takes precedence over configuration file volumes

### Link Management Endpoints

#### List Active Links
```
GET /links
```
Returns all active PipeWire links with detailed connection information.

**Response:**
```json
[
  {
    "id": 101,
    "output_node_id": 45,
    "output_port_id": 67,
    "input_node_id": 89,
    "input_port_id": 90,
    "output_node_name": "speakereq2x2",
    "input_node_name": "HiFiBerry DAC"
  }
]
```

#### Apply Link Rule
```
POST /links/apply
```
Apply a single link rule to create connections between nodes.

**Request Body:**
```json
{
  "name": "My Link",
  "source": {
    "node.name": "^source_node$"
  },
  "destination": {
    "node.name": "^dest_node$"
  },
  "type": "link",
  "link_at_startup": true,
  "relink_every": 0
}
```

**Response:**
```json
{
  "success": true,
  "message": "Created link from output_port to input_port"
}
```

#### Apply Batch Rules
```
POST /links/batch
```
Apply multiple link rules at once.

**Request Body:**
```json
{
  "rules": [
    {
      "name": "Link 1",
      "source": {...},
      "destination": {...},
      "type": "link"
    },
    {
      "name": "Link 2",
      "source": {...},
      "destination": {...},
      "type": "link"
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "message": "Applied 2 rule(s)",
  "details": {
    "results": [
      {"success": true, "message": "Rule applied"},
      {"success": true, "message": "Rule applied"}
    ]
  }
}
```

#### Get Default Link Rules
```
GET /links/default
```
Returns the default link rules configured in the server.

**Response:**
```json
{
  "rules": [
    {
      "name": "SpeakerEQ to HiFiBerry",
      "source": {
        "node.name": "^speakereq2x2\\.output$"
      },
      "destination": {
        "object.path": "alsa:.*:sndrpihifiberry:.*:playback"
      },
      "type": "link",
      "link_at_startup": true,
      "relink_every": 10
    }
  ]
}
```

#### Apply Default Link Rules
```
POST /links/apply-defaults
```
Apply all default link rules configured in the server.

**Response:**
```json
{
  "success": true,
  "message": "Applied 1 default rule(s)",
  "details": {
    "applied": 1,
    "results": [
      {"success": true, "message": "Created 2 links"}
    ]
  }
}
```

#### Get Link Rules Status
```
GET /links/status
```
Returns the status of all configured link rules, including when they last ran and what they did.

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

Status fields:
- `last_run`: ISO 8601 timestamp of the last execution (null if never run)
- `last_run_timestamp`: Unix timestamp of the last execution (null if never run)
- `links_created`: Number of links successfully created on the last run
- `links_failed`: Number of links that failed on the last run
- `last_error`: Last error message, if any (null if no error)
- `total_runs`: Total number of times this rule has been executed


### SpeakerEQ Endpoints

All SpeakerEQ endpoints are prefixed with `/speakereq`.

### Structure Information

#### Get Plugin Structure
```
GET /speakereq/structure
```
Returns the overall structure of the plugin including available blocks and their configuration.

**Response:**
```json
{
  "name": "speakereq2x2",
  "version": "1.0",
  "blocks": [
    {
      "id": "input_0",
      "type": "eq",
      "slots": 20
    },
    {
      "id": "input_1",
      "type": "eq",
      "slots": 20
    },
    {
      "id": "crossbar",
      "type": "crossbar",
      "slots": 4
    },
    {
      "id": "output_0",
      "type": "eq",
      "slots": 20
    },
    {
      "id": "output_1",
      "type": "eq",
      "slots": 20
    },
    {
      "id": "input_gain",
      "type": "volume",
      "slots": 2
    },
    {
      "id": "output_gain",
      "type": "volume",
      "slots": 2
    },
    {
      "id": "master_gain",
      "type": "volume",
      "slots": 1
    }
  ],
  "inputs": 2,
  "outputs": 2,
  "enabled": true,
  "licensed": true
}
```

#### Get Input/Output Count
```
GET /speakereq/io
```
Returns the number of inputs and outputs.

**Response:**
```json
{
  "inputs": 2,
  "outputs": 2
}
```

#### Get Complete Status
```
GET /speakereq/status
```
Get the complete status of the plugin including enable state, master gain, crossbar matrix, and all block configurations with EQ bands in a single call.

**Response:**
```json
{
  "enabled": true,
  "master_gain_db": 0.0,
  "crossbar": {
    "input_0_to_output_0": 1.0,
    "input_0_to_output_1": 0.0,
    "input_1_to_output_0": 0.0,
    "input_1_to_output_1": 1.0
  },
  "inputs": [
    {
      "id": "input_0",
      "type": "input",
      "gain_db": 0.0,
      "eq_bands": [
        {
          "band": 1,
          "type": "off",
          "frequency": 632.45,
          "q": 1.0,
          "gain": 0.0
        }
        // ... bands 2-20
      ]
    },
    {
      "id": "input_1",
      "type": "input",
      "gain_db": 0.0,
      "eq_bands": [
        // ... bands 1-20
      ]
    }
  ],
  "outputs": [
    {
      "id": "output_0",
      "type": "output",
      "gain_db": 0.0,
      "delay_ms": 0.0,
      "eq_bands": [
        {
          "band": 1,
          "type": "peaking",
          "frequency": 1000.0,
          "q": 1.41,
          "gain": 3.0
        }
        // ... bands 2-20
      ]
    },
    {
      "id": "output_1",
      "type": "output",
      "gain_db": 0.0,
      "delay_ms": 0.0,
      "eq_bands": [
        // ... bands 1-20
      ]
    }
  ]
}
```

**Notes:**
- The `delay_ms` field only appears in output blocks, not input blocks
- This endpoint provides a complete snapshot of all plugin settings in one API call
- Useful for initializing UI state or creating backups

---

### EQ Management

#### Get All EQs in a Block
```
GET /speakereq/eq/{block}
```
Get all EQ bands for a specific block (e.g., `input_0`, `output_1`).

**Parameters:**
- `block` (path): Block identifier (`input_0`, `input_1`, `output_0`, `output_1`)

**Response:**
```json
{
  "block": "output_0",
  "eqs": [
    {
      "band": 1,
      "type": "off",
      "frequency": 632.45,
      "q": 1.0,
      "gain": 0.0
    },
    {
      "band": 2,
      "type": "off",
      "frequency": 632.45,
      "q": 1.0,
      "gain": 0.0
    }
    // ... bands 3-20
  ]
}
```

**EQ Types:**
- `off` (0): Off/Bypass
- `low_shelf` (1): Low Shelf
- `high_shelf` (2): High Shelf
- `peaking` (3): Peaking (PEQ)
- `low_pass` (4): Low Pass
- `high_pass` (5): High Pass
- `band_pass` (6): Band Pass
- `notch` (7): Notch
- `all_pass` (8): All Pass

#### Get Single EQ Band
```
GET /speakereq/eq/{block}/{band}
```
Get a specific EQ band configuration.

**Parameters:**
- `block` (path): Block identifier
- `band` (path): Band number (1-20)

**Response:**
```json
{
  "block": "output_0",
  "band": 1,
  "type": "peaking",
  "frequency": 1000.0,
  "q": 1.41,
  "gain": 3.0,
  "enabled": true
}
```

**Fields:**
- `enabled` (boolean, optional): Whether the filter is enabled. If not provided in requests, defaults to `true`.

#### Set Single EQ Band
```
PUT /speakereq/eq/{block}/{band}
```
Update a specific EQ band.

**Parameters:**
- `block` (path): Block identifier
- `band` (path): Band number (1-20)

**Request Body:**
```json
{
  "type": "peaking",
  "frequency": 1000.0,
  "q": 1.41,
  "gain": 3.0,
  "enabled": true
}
```

**Fields:**
- `type` (string, required): EQ type (see EQ Types above)
- `frequency` (float, required): Frequency in Hz (20-20000)
- `q` (float, required): Q factor (0.1-10.0)
- `gain` (float, required): Gain in dB (-24.0 to +24.0)
- `enabled` (boolean, optional): Whether the filter is enabled. If not provided, defaults to `true`

**Response:**
```json
{
  "success": true,
  "block": "output_0",
  "band": 1,
  "updated": {
    "type": "peaking",
    "frequency": 1000.0,
    "q": 1.41,
    "gain": 3.0,
    "enabled": true
  }
}
```

#### Enable/Disable Single EQ Band
```
PUT /speakereq/eq/{block}/{band}/enabled
```
Enable or disable a specific EQ band without modifying its other parameters.

**Parameters:**
- `block` (path): Block identifier
- `band` (path): Band number (1-20)

**Request Body:**
```json
{
  "enabled": false
}
```

**Response:**
```json
{
  "enabled": false
}
```

**Notes:**
- This endpoint only modifies the `enabled` parameter
- All other EQ parameters (type, frequency, Q, gain) remain unchanged
- Useful for temporarily bypassing a filter without losing its configuration

#### Set All EQ Bands in a Block
```
PUT /speakereq/eq/{block}
```
Update all EQ bands in a block at once.

**Parameters:**
- `block` (path): Block identifier

**Request Body:**
```json
{
  "eqs": [
    {
      "band": 1,
      "type": "peaking",
      "frequency": 100.0,
      "q": 1.0,
      "gain": 2.0
    },
    {
      "band": 2,
      "type": "peaking",
      "frequency": 200.0,
      "q": 1.0,
      "gain": -1.5
    }
    // ... can include any or all bands 1-20
  ]
}
```

**Response:**
```json
{
  "success": true,
  "block": "output_0",
  "bandsUpdated": 2
}
```

#### Clear All EQ Bands in a Block
```
PUT /speakereq/eq/{block}/clear
```
Clear all EQ bands in a block by setting them to "off" (type 0).

**Parameters:**
- `block` (path): Block identifier (`input_0`, `input_1`, `output_0`, `output_1`)

**Response:**
```json
{
  "block": "output_0",
  "message": "All EQ bands cleared"
}
```

**Notes:**
- This sets all 20 EQ bands in the specified block to type "off"
- Useful for quickly resetting a block's EQ configuration
- Frequency, Q, and gain values are not changed, only the type is set to "off"

---

### Gain Control

#### Get All Gains
```
GET /gain
```
Get all gain values (input, output, master).

**Response:**
```json
{
  "master": 0.0,
  "inputs": [
    { "channel": 0, "gain": 0.0 },
    { "channel": 1, "gain": 0.0 }
  ],
  "outputs": [
    { "channel": 0, "gain": 0.0 },
    { "channel": 1, "gain": 0.0 }
  ]
}
```

#### Get Master Gain
```
GET /speakereq/gain/master
```
**Response:**
```json
{
  "gain": 0.0
}
```

#### Set Master Gain
```
PUT /speakereq/gain/master
```
**Request Body:**
```json
{
  "gain": -3.0
}
```

**Response:**
```json
{
  "success": true,
  "gain": -3.0
}
```

#### Get Input Gain
```
GET /speakereq/gain/input/{channel}
```
**Parameters:**
- `channel` (path): Channel number (0-1)

**Response:**
```json
{
  "channel": 0,
  "gain": 0.0
}
```

#### Set Input Gain
```
PUT /speakereq/gain/input/{channel}
```
**Request Body:**
```json
{
  "gain": 2.0
}
```

**Response:**
```json
{
  "success": true,
  "channel": 0,
  "gain": 2.0
}
```

#### Get Output Gain
```
GET /speakereq/gain/output/{channel}
```
**Parameters:**
- `channel` (path): Channel number (0-1)

**Response:**
```json
{
  "channel": 0,
  "gain": 0.0
}
```

#### Set Output Gain
```
PUT /speakereq/gain/output/{channel}
```
**Request Body:**
```json
{
  "gain": -1.5
}
```

**Response:**
```json
{
  "success": true,
  "channel": 0,
  "gain": -1.5
}
```

---

### Delay Control

#### Get All Delays
```
GET /speakereq/delay
```
**Response:**
```json
{
  "delays": [
    { "channel": 0, "ms": 0.0 },
    { "channel": 1, "ms": 0.0 }
  ]
}
```

#### Set Delay
```
PUT /speakereq/delay/{channel}
```
**Request Body:**
```json
{
  "ms": 2.5
}
```

**Response:**
```json
{
  "success": true,
  "channel": 0,
  "ms": 2.5
}
```

---

### Crossbar Matrix

#### Get Crossbar Matrix
```
GET /speakereq/crossbar
```
Get the routing matrix values.

**Response:**
```json
{
  "matrix": [
    [1.0, 0.0],  // input 0 -> outputs [0, 1]
    [0.0, 1.0]   // input 1 -> outputs [0, 1]
  ]
}
```

#### Set Crossbar Value
```
PUT /speakereq/crossbar/{input}/{output}
```
Set a single crossbar routing value.

**Parameters:**
- `input` (path): Input channel (0-1)
- `output` (path): Output channel (0-1)

**Request Body:**
```json
{
  "value": 0.5
}
```

**Response:**
```json
{
  "success": true,
  "input": 0,
  "output": 0,
  "value": 0.5
}
```

---

### Global Control

#### Get Enable Status
```
GET /speakereq/enable
```
**Response:**
```json
{
  "enabled": true,
  "licensed": true
}
```

#### Set Enable Status
```
PUT /speakereq/enable
```
**Request Body:**
```json
{
  "enabled": false
}
```

**Response:**
```json
{
  "success": true,
  "enabled": false
}
```

#### Get License Status
```
GET /speakereq/license
```
**Response:**
```json
{
  "licensed": true
}
```

---

### Settings Persistence

#### Save Current Settings
```
POST /speakereq/settings/save
```
Trigger saving current settings to file.

**Response:**
```json
{
  "success": true,
  "message": "Settings saved"
}
```

#### Get All Parameters
```
GET /speakereq/parameters
```
Get all raw parameters (for advanced users/debugging).

**Response:**
```json
{
  "speakereq2x2:Enable": true,
  "speakereq2x2:master_gain_db": 0.0,
  "speakereq2x2:input_0_eq_1_type": 0,
  // ... all parameters
}
```

---

## Error Responses

All endpoints may return error responses:

**400 Bad Request:**
```json
{
  "error": "Invalid parameter value",
  "message": "Frequency must be between 20 and 20000 Hz"
}
```

**404 Not Found:**
```json
{
  "error": "Block not found",
  "message": "Block 'input_3' does not exist"
}
```

**500 Internal Server Error:**
```json
{
  "error": "PipeWire error",
  "message": "Failed to communicate with audio node"
}
```

---

## Implementation Notes

### Technology Stack
- **Web Framework**: Axum (Rust async web framework)
- **JSON Serialization**: serde_json
- **PipeWire Interface**: Reuse existing pw-param logic

### Parameter Naming Convention
The plugin uses the following parameter naming:
- EQ: `speakereq2x2:{block}_eq_{band}_{property}`
  - Example: `speakereq2x2:output_0_eq_1_gain`
- Gains: `speakereq2x2:{type}_gain_{channel}_db`
  - Example: `speakereq2x2:input_gain_0_db`
- Crossbar: `speakereq2x2:xbar_{input}_to_{output}`
  - Example: `speakereq2x2:xbar_0_to_1`
- Delay: `speakereq2x2:delay_{channel}_ms`

### Validation Rules
- **Frequency**: 20 - 20000 Hz
- **Q Factor**: 0.1 - 10.0
- **Gain**: -24 to +24 dB
- **EQ Type**: One of: `off`, `low_shelf`, `high_shelf`, `peaking`, `low_pass`, `high_pass`, `band_pass`, `notch`, `all_pass`
- **Band Number**: 1-20
- **Channel Number**: 0-1
- **Crossbar Value**: 0.0 - 2.0
- **Delay**: 0 - 100 ms

### EQ Type Mapping (Internal)
When communicating with PipeWire, the API converts between string names and integer values:
```
off        = 0
low_shelf  = 1
high_shelf = 2
peaking    = 3
low_pass   = 4
high_pass  = 5
band_pass  = 6
notch      = 7
all_pass   = 8
```
