# PipeWire API - Link Management

Link management uses pw-link for creating and managing PipeWire audio connections. Links can be identified by port IDs or port names.

## Base URL
`http://localhost:2716/api/v1`

---

## List Active Links

```
GET /api/v1/links
```

Returns all active PipeWire links with port information.

**Response:**
```json
{
  "links": [
    {
      "id": 101,
      "output_port_id": 67,
      "output_port_name": "speakereq2x2:output_FL",
      "input_port_id": 89,
      "input_port_name": "alsa_output.platform-soc_audio.stereo-fallback:playback_FL"
    }
  ]
}
```

---

## Create Link

```
POST /api/v1/links
```

Create a link between two ports. Ports can be specified by ID or name (format: "node_name:port_name").

**Request Body (by name):**
```json
{
  "output": "speakereq2x2:output_FL",
  "input": "alsa_output.platform-soc_audio.stereo-fallback:playback_FL"
}
```

**Request Body (by ID):**
```json
{
  "output": "67",
  "input": "89"
}
```

**Response:**
```json
{
  "status": "ok",
  "message": "Link created: speakereq2x2:output_FL -> alsa_output.platform-soc_audio.stereo-fallback:playback_FL",
  "link_id": 101
}
```

---

## Remove Link by ID

```
DELETE /api/v1/links/:id
```

Remove a link by its link ID.

**Parameters:**
- `id` (path): Link ID

**Response:**
```json
{
  "status": "ok",
  "message": "Link 101 removed",
  "link_id": 101
}
```

---

## Remove Link by Name

```
DELETE /api/v1/links/by-name
```

Remove a link by specifying the output and input ports.

**Request Body:**
```json
{
  "output": "speakereq2x2:output_FL",
  "input": "alsa_output.platform-soc_audio.stereo-fallback:playback_FL"
}
```

**Response:**
```json
{
  "status": "ok",
  "message": "Link removed: speakereq2x2:output_FL -> alsa_output.platform-soc_audio.stereo-fallback:playback_FL"
}
```

---

## Check if Link Exists

```
GET /api/v1/links/exists?output=...&input=...
```

Check if a link exists between two ports.

**Query Parameters:**
- `output`: Output port name or ID
- `input`: Input port name or ID

**Response (exists):**
```json
{
  "exists": true,
  "link_id": 101
}
```

**Response (not exists):**
```json
{
  "exists": false
}
```

---

## List Output Ports

```
GET /api/v1/links/ports/output
```

Returns all available output (playback) ports.

**Response:**
```json
{
  "ports": [
    {
      "id": 67,
      "name": "speakereq2x2:output_FL",
      "node_name": "speakereq2x2",
      "port_name": "output_FL"
    }
  ]
}
```

---

## List Input Ports

```
GET /api/v1/links/ports/input
```

Returns all available input (capture) ports.

**Response:**
```json
{
  "ports": [
    {
      "id": 89,
      "name": "alsa_output.platform-soc_audio.stereo-fallback:playback_FL",
      "node_name": "alsa_output.platform-soc_audio.stereo-fallback",
      "port_name": "playback_FL"
    }
  ]
}
```

---

## Link Rules (Experimental)

Additional endpoints for rule-based link management:

### Get Default Link Rules
```
GET /api/v1/links/default
```

### Apply Default Rules
```
POST /api/v1/links/apply-defaults
```

### Apply Single Rule
```
POST /api/v1/links/apply
```

### Apply Batch Rules
```
POST /api/v1/links/batch
```

### Get Link Rules Status
```
GET /api/v1/links/status
```

See [LINKS_API.md](LINKS_API.md) for detailed documentation on link rules.
