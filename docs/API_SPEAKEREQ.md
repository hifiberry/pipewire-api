# PipeWire API - SpeakerEQ Module

The SpeakerEQ module provides control for the SpeakerEQ 2x2 audio filter plugin, including parametric EQ, gain control, crossbar routing, and delay settings.

## Base URL
`http://localhost:2716/api/v1/module/speakereq`

---

## Structure Information

### Get Plugin Structure

```
GET /api/v1/module/speakereq/structure
```

Returns the overall structure of the plugin including available blocks and their configuration.

**Response:**
```json
{
  "name": "speakereq2x2",
  "version": "1.0",
  "blocks": [
    { "id": "input_0", "type": "eq", "slots": 20 },
    { "id": "input_1", "type": "eq", "slots": 20 },
    { "id": "crossbar", "type": "crossbar", "slots": 4 },
    { "id": "output_0", "type": "eq", "slots": 20 },
    { "id": "output_1", "type": "eq", "slots": 20 },
    { "id": "input_gain", "type": "volume", "slots": 2 },
    { "id": "output_gain", "type": "volume", "slots": 2 },
    { "id": "master_gain", "type": "volume", "slots": 1 }
  ],
  "inputs": 2,
  "outputs": 2,
  "enabled": true,
  "licensed": true
}
```

### Get Input/Output Count

```
GET /api/v1/module/speakereq/io
```

Returns the number of inputs and outputs.

**Response:**
```json
{
  "inputs": 2,
  "outputs": 2
}
```

### Get Plugin Configuration (Dynamic)

```
GET /api/v1/module/speakereq/config
```

Dynamically discovers the plugin configuration by probing available parameters from PipeWire.

**Response:**
```json
{
  "inputs": 2,
  "outputs": 2,
  "eq_slots": {
    "input_0": 20,
    "input_1": 20,
    "output_0": 20,
    "output_1": 20
  },
  "plugin_name": "speakereq2x2",
  "method": "probed_from_parameters"
}
```

### Get Complete Status

```
GET /api/v1/module/speakereq/status
```

Get the complete status of the plugin including enable state, master gain, crossbar matrix, and all block configurations with EQ bands.

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
        { "band": 1, "type": "off", "frequency": 632.45, "q": 1.0, "gain": 0.0 }
        // ... bands 2-20
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
        { "band": 1, "type": "peaking", "frequency": 1000.0, "q": 1.41, "gain": 3.0 }
        // ... bands 2-20
      ]
    }
  ]
}
```

---

## EQ Management

### EQ Types

| Type | ID | Description |
|------|----|-------------|
| `off` | 0 | Off/Bypass |
| `low_shelf` | 1 | Low Shelf |
| `high_shelf` | 2 | High Shelf |
| `peaking` | 3 | Peaking (PEQ) |
| `low_pass` | 4 | Low Pass |
| `high_pass` | 5 | High Pass |
| `band_pass` | 6 | Band Pass |
| `notch` | 7 | Notch |
| `all_pass` | 8 | All Pass |

### Get Single EQ Band

```
GET /api/v1/module/speakereq/eq/:block/:band
```

Get a specific EQ band configuration.

**Parameters:**
- `block`: Block identifier (`input_0`, `input_1`, `output_0`, `output_1`)
- `band`: Band number (1-20)

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

### Set Single EQ Band

```
PUT /api/v1/module/speakereq/eq/:block/:band
```

Update a specific EQ band.

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

### Enable/Disable Single EQ Band

```
PUT /api/v1/module/speakereq/eq/:block/:band/enabled
```

Enable or disable a specific EQ band without modifying its other parameters.

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

### Clear All EQ Bands in a Block

```
PUT /api/v1/module/speakereq/eq/:block/clear
```

Clear all EQ bands in a block by setting them to "off" (type 0).

**Response:**
```json
{
  "block": "output_0",
  "message": "All EQ bands cleared"
}
```

---

## Gain Control

### Get Master Gain

```
GET /api/v1/module/speakereq/gain/master
```

**Response:**
```json
{
  "gain": 0.0
}
```

### Set Master Gain

```
PUT /api/v1/module/speakereq/gain/master
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

### Get/Set Input Gain

```
GET /api/v1/module/speakereq/gain/input/:channel
PUT /api/v1/module/speakereq/gain/input/:channel
```

**Parameters:**
- `channel`: Channel number (0-1)

### Get/Set Output Gain

```
GET /api/v1/module/speakereq/gain/output/:channel
PUT /api/v1/module/speakereq/gain/output/:channel
```

**Parameters:**
- `channel`: Channel number (0-1)

---

## Delay Control

### Get All Delays

```
GET /api/v1/module/speakereq/delay
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

### Set Delay

```
PUT /api/v1/module/speakereq/delay/:channel
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

## Crossbar Matrix

### Get Crossbar Matrix

```
GET /api/v1/module/speakereq/crossbar
```

Get the routing matrix values.

**Response:**
```json
{
  "matrix": [
    [1.0, 0.0],
    [0.0, 1.0]
  ]
}
```

### Set Entire Crossbar Matrix

```
PUT /api/v1/module/speakereq/crossbar
```

Set all crossbar routing values in a single request.

**Request Body:**
```json
{
  "matrix": [
    [0.8, 0.2],
    [0.3, 0.7]
  ]
}
```

**Response:**
```json
{
  "success": true,
  "matrix": [
    [0.8, 0.2],
    [0.3, 0.7]
  ]
}
```

**Notes:**
- Matrix must be exactly 2x2 (2 inputs × 2 outputs)
- All values must be between 0.0 and 2.0
- More efficient than setting individual values
- All values updated atomically

### Set Crossbar Value

```
PUT /api/v1/module/speakereq/crossbar/:input/:output
```

Set a single crossbar routing value.

**Parameters:**
- `input`: Input channel (0-1)
- `output`: Output channel (0-1)

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

## Global Control

### Get Enable Status

```
GET /api/v1/module/speakereq/enable
```

**Response:**
```json
{
  "enabled": true,
  "licensed": true
}
```

### Set Enable Status

```
PUT /api/v1/module/speakereq/enable
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

### Refresh Parameter Cache

```
POST /api/v1/module/speakereq/refresh
```

Force refresh of parameter cache.

**Response:**
```json
{
  "message": "Parameter cache refreshed"
}
```

### Set Default Configuration

```
POST /api/v1/module/speakereq/default
```

Reset all parameters to default values:
- All gains set to 0dB
- Crossbar matrix set to identity
- All EQ filters set to "off"
- Enable status set to true

**Response:**
```json
{
  "status": "ok",
  "message": "All parameters set to default values"
}
```

### Get License Status

```
GET /api/v1/module/speakereq/license
```

**Response:**
```json
{
  "licensed": true
}
```

---

## Validation Rules

| Parameter | Range |
|-----------|-------|
| Frequency | 20 - 20000 Hz |
| Q Factor | 0.1 - 10.0 |
| Gain | -24 to +24 dB |
| Band Number | 1-20 |
| Channel Number | 0-1 |
| Crossbar Value | 0.0 - 2.0 |
| Delay | 0 - 100 ms |

---

## Parameter Naming Convention

Internal parameter names used by PipeWire:
- EQ: `speakereq2x2:{block}_eq_{band}_{property}` (e.g., `speakereq2x2:output_0_eq_1_gain`)
- Gains: `speakereq2x2:{type}_gain_{channel}_db` (e.g., `speakereq2x2:input_gain_0_db`)
- Crossbar: `speakereq2x2:xbar_{input}_to_{output}` (e.g., `speakereq2x2:xbar_0_to_1`)
- Delay: `speakereq2x2:delay_{channel}_ms`
