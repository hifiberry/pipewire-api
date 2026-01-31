# SpeakerEQ 2x2 REST API Specification

## Overview
REST API for controlling the SpeakerEQ 2x2 PipeWire audio filter plugin.

## Base URL
`http://localhost:2716/api/v1`

Note: The server binds to all interfaces (0.0.0.0) by default. Use `--localhost` flag to restrict to localhost only.

## Endpoints

### Structure Information

#### Get Plugin Structure
```
GET /structure
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
GET /io
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
GET /status
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
GET /eq/{block}
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
GET /eq/{block}/{band}
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
PUT /eq/{block}/{band}
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
PUT /eq/{block}/{band}/enabled
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
PUT /eq/{block}
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
PUT /eq/{block}/clear
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
GET /gain/master
```
**Response:**
```json
{
  "gain": 0.0
}
```

#### Set Master Gain
```
PUT /gain/master
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
GET /gain/input/{channel}
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
PUT /gain/input/{channel}
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
GET /gain/output/{channel}
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
PUT /gain/output/{channel}
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
GET /delay
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
PUT /delay/{channel}
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
GET /crossbar
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
PUT /crossbar/{input}/{output}
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
GET /enable
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
PUT /enable
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
GET /license
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
POST /settings/save
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
GET /parameters
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
