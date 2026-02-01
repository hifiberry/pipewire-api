# PipeWire REST API Specification

## Overview

REST API for controlling PipeWire audio system, with specialized support for the SpeakerEQ 2x2 audio filter plugin and RIAA phono preamplifier.

Includes endpoints for:
- Listing PipeWire objects and inspecting properties
- Volume control for devices and sinks
- Managing audio links
- Graph visualization of audio topology
- Controlling SpeakerEQ parameters (EQ, gain, delay, crossbar)
- Controlling RIAA phono preamplifier

## Documentation

For detailed endpoint documentation, see the module-specific files:

| Module | Description | Documentation |
|--------|-------------|---------------|
| **Core** | Object listing, properties, cache management | [docs/API_CORE.md](docs/API_CORE.md) |
| **Volume** | Unified volume control for devices and sinks | [docs/API_VOLUME.md](docs/API_VOLUME.md) |
| **Links** | PipeWire link management | [docs/API_LINKS.md](docs/API_LINKS.md) |
| **Graph** | Visual topology graphs (DOT/PNG) | [docs/API_GRAPH.md](docs/API_GRAPH.md) |
| **SpeakerEQ** | Parametric EQ, gain, delay, crossbar | [docs/API_SPEAKEREQ.md](docs/API_SPEAKEREQ.md) |
| **RIAA** | Phono preamplifier control | [docs/API_RIAA.md](docs/API_RIAA.md) |

## Base URLs

| Module | Base URL |
|--------|----------|
| Core, Volume, Links, Graph | `http://localhost:2716/api/v1` |
| SpeakerEQ | `http://localhost:2716/api/v1/module/speakereq` |
| RIAA | `http://localhost:2716/api/v1/module/riaa` |

Note: The server binds to all interfaces (0.0.0.0) by default. Use `--localhost` flag to restrict to localhost only.

## Quick Reference

### Core Endpoints (`/api/v1`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1` | GET | List all API endpoints |
| `/api/v1/ls` | GET | List all PipeWire objects |
| `/api/v1/objects/:id` | GET | Get object by ID |
| `/api/v1/cache/refresh` | POST | Refresh object cache |
| `/api/v1/properties` | GET | List all objects with properties |
| `/api/v1/properties/:id` | GET | Get object properties by ID |

### Graph Endpoints (`/api/v1`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1/graph` | GET | Get audio topology in DOT format |
| `/api/v1/graph/png` | GET | Get audio topology as PNG image |

### Volume Endpoints (`/api/v1/volume`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1/volume` | GET | List all volumes |
| `/api/v1/volume/:id` | GET, PUT | Get/set volume by ID |
| `/api/v1/volume/save` | POST | Save all volumes |
| `/api/v1/volume/save/:id` | POST | Save specific volume |

### Link Endpoints (`/api/v1/links`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1/links` | GET, POST | List/create links |
| `/api/v1/links/:id` | DELETE | Remove link by ID |
| `/api/v1/links/by-name` | DELETE | Remove link by port names |
| `/api/v1/links/exists` | GET | Check if link exists |
| `/api/v1/links/ports/output` | GET | List output ports |
| `/api/v1/links/ports/input` | GET | List input ports |

### SpeakerEQ Endpoints (`/api/v1/module/speakereq`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1/module/speakereq/structure` | GET | Get DSP structure |
| `/api/v1/module/speakereq/config` | GET | Get configuration |
| `/api/v1/module/speakereq/io` | GET | Get I/O count |
| `/api/v1/module/speakereq/status` | GET | Get complete status |
| `/api/v1/module/speakereq/eq/:block/:band` | GET, PUT | Get/set EQ band |
| `/api/v1/module/speakereq/eq/:block/:band/enabled` | PUT | Enable/disable EQ band |
| `/api/v1/module/speakereq/eq/:block/clear` | PUT | Clear all EQ in block |
| `/api/v1/module/speakereq/gain/master` | GET, PUT | Get/set master gain |
| `/api/v1/module/speakereq/enable` | GET, PUT | Get/set enable status |
| `/api/v1/module/speakereq/refresh` | POST | Refresh parameter cache |
| `/api/v1/module/speakereq/default` | POST | Reset to defaults |

### RIAA Endpoints (`/api/v1/module/riaa`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1/module/riaa/config` | GET | Get all RIAA settings |
| `/api/v1/module/riaa/gain` | GET, PUT | Get/set gain |
| `/api/v1/module/riaa/bass` | GET, PUT | Get/set bass |
| `/api/v1/module/riaa/treble` | GET, PUT | Get/set treble |
| `/api/v1/module/riaa/balance` | GET, PUT | Get/set balance |
| `/api/v1/module/riaa/loudness` | GET, PUT | Get/set loudness |
| `/api/v1/module/riaa/enable` | GET, PUT | Get/set enable status |
| `/api/v1/module/riaa/refresh` | POST | Refresh parameter cache |
| `/api/v1/module/riaa/default` | POST | Reset to defaults |

## Getting Started

### List All Endpoints
```bash
curl http://localhost:2716/api/v1
```

### List All PipeWire Objects
```bash
curl http://localhost:2716/api/v1/ls
```

### Get Volume Status
```bash
curl http://localhost:2716/api/v1/volume
```

### View Audio Topology
```bash
# DOT format
curl http://localhost:2716/api/v1/graph

# PNG image (requires graphviz)
curl -o graph.png http://localhost:2716/api/v1/graph/png
```

## Error Handling

All endpoints return standard HTTP status codes:

| Status | Description |
|--------|-------------|
| 200 | Success |
| 400 | Bad Request (invalid parameters) |
| 404 | Not Found (object/resource doesn't exist) |
| 500 | Internal Server Error |

Error responses include a JSON body:
```json
{
  "error": "Description of the error"
}
```
