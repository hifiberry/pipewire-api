# PipeWire API - Core Endpoints

Core endpoints for listing PipeWire objects, inspecting properties, and managing the object cache.

## Base URL
`http://localhost:2716/api/v1`

Note: The server binds to all interfaces (0.0.0.0) by default. Use `--localhost` flag to restrict to localhost only.

---

## List All API Endpoints

```
GET /api/v1
```

Returns a list of all available API endpoints with their methods and descriptions.

**Response:**
```json
{
  "version": "1.0",
  "endpoints": [
    {
      "path": "/api/v1",
      "methods": ["GET"],
      "description": "List all available API endpoints"
    },
    {
      "path": "/api/v1/ls",
      "methods": ["GET"],
      "description": "List all PipeWire objects"
    }
    // ... more endpoints
  ]
}
```

---

## List All Objects

```
GET /api/v1/ls
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

**Note:** Filter results client-side by checking the `type` field in the response. Available types include: `node`, `device`, `port`, `link`, `client`, `module`, `factory`.

---

## Get Object by ID

```
GET /api/v1/objects/:id
```

Returns a single object by its ID.

**Parameters:**
- `id` (path): Object ID

**Response:**
```json
{
  "id": 45,
  "name": "speakereq2x2",
  "type": "node"
}
```

**Error Response:**
- `404 Not Found` if object doesn't exist

---

## Refresh Object Cache

```
POST /api/v1/cache/refresh
```

Forces a refresh of the internal PipeWire object cache. The cache is automatically updated on startup and can be refreshed manually using this endpoint.

**Response:**
```json
{
  "status": "ok",
  "message": "Cache refreshed",
  "object_count": 127
}
```

---

## Get All Properties

```
GET /api/v1/properties
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

---

## Get Object Properties by ID

```
GET /api/v1/properties/:id
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
  "error": "Object not found",
  "message": "Object with ID 999 does not exist"
}
```

**500 Internal Server Error:**
```json
{
  "error": "PipeWire error",
  "message": "Failed to communicate with audio node"
}
```
