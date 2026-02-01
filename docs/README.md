# PipeWire REST API Documentation

REST API for controlling PipeWire audio system, with specialized support for audio processing plugins.

## Quick Start

The API server runs on port 2716 by default. Get a list of all endpoints:

```bash
curl http://localhost:2716/api/v1
```

## API Documentation

The API is organized into the following modules:

| Module | Description | Documentation |
|--------|-------------|---------------|
| **Core** | Object listing, properties, cache management | [API_CORE.md](API_CORE.md) |
| **Volume** | Unified volume control for devices and sinks | [API_VOLUME.md](API_VOLUME.md) |
| **Links** | PipeWire link management | [API_LINKS.md](API_LINKS.md) |
| **Graph** | Visual topology graphs (DOT/PNG) | [API_GRAPH.md](API_GRAPH.md) |
| **SpeakerEQ** | Parametric EQ, gain, delay, crossbar | [API_SPEAKEREQ.md](API_SPEAKEREQ.md) |
| **RIAA** | Phono preamplifier control | [API_RIAA.md](API_RIAA.md) |

## Base URLs

- Core/Volume/Links: `http://localhost:2716/api/v1`
- SpeakerEQ: `http://localhost:2716/api/v1/module/speakereq`
- RIAA: `http://localhost:2716/api/v1/module/riaa`

## Endpoint Summary

### Core Endpoints (`/api/v1`)
| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/v1` | GET | List all API endpoints |
| `/api/v1/ls` | GET | List all PipeWire objects |
| `/api/v1/objects/:id` | GET | Get object by ID |
| `/api/v1/cache/refresh` | POST | Refresh object cache |
| `/api/v1/properties` | GET | List all objects with properties |
| `/api/v1/properties/:id` | GET | Get object properties by ID |
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
| `/api/v1/module/riaa/subsonic` | GET, PUT | Get/set subsonic filter |
| `/api/v1/module/riaa/riaa-enable` | GET, PUT | Enable/disable RIAA EQ |
| `/api/v1/module/riaa/declick` | GET, PUT | Enable/disable declicker |
| `/api/v1/module/riaa/spike` | GET, PUT | Get/set spike detection |
| `/api/v1/module/riaa/notch` | GET, PUT | Get/set notch filter |
| `/api/v1/module/riaa/set-default` | PUT | Reset to defaults |

## Error Responses

All endpoints may return error responses:

```json
{
  "error": "Error type",
  "message": "Detailed error message"
}
```

| Status Code | Description |
|-------------|-------------|
| 400 | Bad Request - Invalid parameters |
| 404 | Not Found - Object/resource not found |
| 500 | Internal Server Error - PipeWire communication error |

## Server Configuration

The server binds to all interfaces (0.0.0.0) by default on port 2716.

```bash
# Start with default settings
pipewire-api

# Restrict to localhost only
pipewire-api --localhost

# Custom port
pipewire-api --port 8080
```
