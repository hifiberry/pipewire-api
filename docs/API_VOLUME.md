# PipeWire API - Volume Management

Unified volume control for both hardware devices and software sinks (audio outputs). The API automatically detects the object type and uses the appropriate PipeWire parameters via wpctl.

## Base URL
`http://localhost:2716/api/v1`

---

## List All Volumes

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
- `object_type`: Either `"device"` (hardware device), `"sink"` (software audio output), or `"source"` (software audio input)
- `volume`: Float value where 1.0 = 100%, 0.5 = 50%, etc. (only present if volume is available)

**Notes:**
- Only objects with an actual volume setting are returned
- Objects without volume control are automatically filtered out

---

## Get Volume by ID

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

---

## Set Volume by ID

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

---

## Save All Volumes

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

---

## Save Specific Volume

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
  "name": "alsa_output.pci-0000_00_1f.3.analog-stereo",
  "volume": 0.85,
  "message": "Volume state saved"
}
```

---

## Volume State Persistence

- Saved volumes persist across restarts when `use_state_file: true` is set in volume.conf
- State file location: `~/.state/pipewire-api/volume.state`
- State file takes precedence over configuration file volumes
