# PipeWire API - RIAA Phono Preamplifier Module

The RIAA module provides control for the vinyl phono preamplifier plugin, including RIAA equalization, subsonic filtering, declicking, and notch filtering.

## Base URL
`http://localhost:2716/api/module/riaa`

---

## Get Complete Configuration

```
GET /api/module/riaa/config
```

Returns all RIAA settings in a single response.

**Response:**
```json
{
  "gain_db": 0.0,
  "subsonic_filter": 0,
  "riaa_enable": true,
  "declick_enable": false,
  "spike_threshold_db": 25.0,
  "spike_width_ms": 2.0,
  "notch_filter_enable": false,
  "notch_frequency_hz": 300.0,
  "notch_q_factor": 30.0
}
```

---

## Get/Set Gain

```
GET /api/module/riaa/gain
PUT /api/module/riaa/gain
```

Get or set the preamplifier gain in decibels.

**GET Response:**
```json
{
  "gain_db": 0.0
}
```

**PUT Request:**
```json
{
  "gain_db": 3.5
}
```

**PUT Response:**
```json
{
  "success": true,
  "gain_db": 3.5
}
```

---

## Get/Set Subsonic Filter

```
GET /api/module/riaa/subsonic
PUT /api/module/riaa/subsonic
```

Get or set the subsonic (rumble) filter setting.

**GET Response:**
```json
{
  "filter": 0
}
```

**PUT Request:**
```json
{
  "filter": 1
}
```

**Filter Values:**
| Value | Cutoff Frequency |
|-------|------------------|
| `0`   | Off              |
| `1`   | 20 Hz            |
| `2`   | 30 Hz            |
| `3`   | 40 Hz            |

**PUT Response:**
```json
{
  "success": true,
  "filter": 1
}
```

---

## Get/Set RIAA Enable

```
GET /api/module/riaa/riaa-enable
PUT /api/module/riaa/riaa-enable
```

Enable or disable the RIAA equalization curve.

**GET Response:**
```json
{
  "enabled": true
}
```

**PUT Request:**
```json
{
  "enabled": false
}
```

**PUT Response:**
```json
{
  "success": true,
  "enabled": false
}
```

---

## Get/Set Declick Enable

```
GET /api/module/riaa/declick
PUT /api/module/riaa/declick
```

Enable or disable the declicker for removing pops and clicks from vinyl playback.

**GET Response:**
```json
{
  "enabled": false
}
```

**PUT Request:**
```json
{
  "enabled": true
}
```

**PUT Response:**
```json
{
  "success": true,
  "enabled": true
}
```

---

## Get/Set Spike Detection Configuration

```
GET /api/module/riaa/spike
PUT /api/module/riaa/spike
```

Configure the spike detection parameters for the declicker.

**GET Response:**
```json
{
  "threshold_db": 25.0,
  "width_ms": 2.0
}
```

**PUT Request:**
```json
{
  "threshold_db": 20.0,
  "width_ms": 1.5
}
```

**Parameters:**
| Parameter | Description | Typical Range |
|-----------|-------------|---------------|
| `threshold_db` | Spike detection threshold in dB | 15-30 dB |
| `width_ms` | Maximum spike width in milliseconds | 0.5-5.0 ms |

**PUT Response:**
```json
{
  "success": true,
  "threshold_db": 20.0,
  "width_ms": 1.5
}
```

---

## Get/Set Notch Filter Configuration

```
GET /api/module/riaa/notch
PUT /api/module/riaa/notch
```

Configure the notch filter for removing specific frequencies (e.g., turntable motor noise at 50Hz or 60Hz).

**GET Response:**
```json
{
  "enabled": false,
  "frequency_hz": 300.0,
  "q_factor": 30.0
}
```

**PUT Request:**
```json
{
  "enabled": true,
  "frequency_hz": 50.0,
  "q_factor": 10.0
}
```

**Parameters:**
| Parameter | Description |
|-----------|-------------|
| `enabled` | Whether the notch filter is active |
| `frequency_hz` | Center frequency of the notch in Hz |
| `q_factor` | Q factor (higher = narrower notch) |

**PUT Response:**
```json
{
  "success": true,
  "enabled": true,
  "frequency_hz": 50.0,
  "q_factor": 10.0
}
```

---

## Reset to Defaults

```
PUT /api/module/riaa/set-default
```

Reset all RIAA parameters to their default values.

**Default Values:**
| Parameter | Default Value |
|-----------|---------------|
| `gain_db` | 0.0 |
| `subsonic_filter` | 0 (off) |
| `riaa_enable` | true |
| `declick_enable` | false |
| `spike_threshold_db` | 20.0 |
| `spike_width_ms` | 1.0 |
| `notch_filter_enable` | false |
| `notch_frequency_hz` | 50.0 |
| `notch_q_factor` | 10.0 |

**Response:**
```json
{
  "status": "ok",
  "message": "RIAA parameters reset to defaults"
}
```
