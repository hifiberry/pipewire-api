# Volume Configuration

Configuration file for setting default volumes on PipeWire devices and sinks at server startup.

## Configuration File Locations

The server looks for volume configuration in the following locations (in order of precedence):

1. **User config**: `~/.config/pipewire-api/volume.conf` (highest priority)
2. **System config**: `/etc/pipewire-api/volume.conf` (fallback)

## Configuration Format

The configuration file is a JSON array of volume rules:

```json
[
  {
    "name": "HiFiBerry DAC Default Volume",
    "object": {
      "device.name": "alsa_card\\.platform-soc_.*_sound"
    },
    "volume": 0.75
  }
]
```

### Rule Properties

- **name**: Human-readable description of the rule
- **object**: Object containing matching criteria (key-value pairs)
  - Keys are PipeWire property names (device.* for devices, node.* for sinks)
  - Values are regex patterns to match against
  - All specified properties must match for the rule to apply
- **volume**: Volume level to set (0.0 = mute, 1.0 = 100%, 2.0 = 200%, values >1.0 may cause clipping)
- **use_state_file**: (optional, default: false) If true, use volume from state file if available instead of config volume

## Volume State File

When `use_state_file: true` is set in a rule, the server checks for saved volumes in:
- `~/.state/pipewire-api/volume.state`

The state file uses object names (e.g., `device.name` or `node.name`) as keys to identify objects. If a volume is found for a matching object in the state file, it takes precedence over the volume in the configuration file. This allows volumes to persist across restarts after being changed via the API.

**State file format:**
```json
[
  {
    "name": "alsa_card.platform-soc_107c000000_sound",
    "volume": 0.75
  },
  {
    "name": "speakereq2x2",
    "volume": 0.85
  }
]
```

**Saving volumes:**
```bash
# Save all current volumes
curl -X POST http://localhost:2716/api/v1/volume/save

# Save specific volume (by ID, saves using name as key)
curl -X POST http://localhost:2716/api/v1/volume/save/56
```

## Object Matching

Properties are matched using regular expressions. The configuration works for both devices and sinks.

### Device Properties
Common device properties include:
- `device.name`: Device name (e.g., `alsa_card.usb-Audio_Device`)
- `device.description`: Human-readable description
- `device.api`: API used (e.g., `alsa`, `v4l2`)

### Sink Properties  
Common sink (node) properties include:
- `node.name`: Sink name (e.g., `alsa_output.pci-0000_00_1f.3.analog-stereo`)
- `node.description`: Human-readable description
- `media.class`: Should be `Audio/Sink` for sinks

Use `pw-dump` or `pw-cli ls Device` / `pw-cli ls Node` to see available objects and their properties.

## Examples

### Set volume for all ALSA devices:
```json
{
  "name": "All ALSA Devices",
  "object": {
    "device.api": "alsa"
  },
  "volume": 0.8
}
```

### Set volume for USB audio devices:
```json
{
  "name": "USB Audio Devices",
  "object": {
    "device.name": "alsa_card\\.usb-.*"
  },
  "volume": 0.9
}
```

### Set volume for specific device with state file:
```json
{
  "name": "HiFiBerry DAC+ ADC",
  "object": {
    "device.name": "alsa_card\\.platform-soc_.*_sound",
    "device.nick": "snd_rpi_hifiberry_dacplusadc"
  },
  "volume": 0.75,
  "use_state_file": true
}
```
With `use_state_file: true`, if you save the volume via the API, the saved value will be used on next startup instead of 0.75.

### Set volume for audio sinks:
```json
{
  "name": "Built-in Audio Sink",
  "object": {
    "node.name": "alsa_output\\.pci-.*\\.analog-stereo",
    "media.class": "Audio/Sink"
  },
  "volume": 0.85
}
```

### Set volume for SpeakerEQ plugin:
```json
{
  "name": "SpeakerEQ Output",
  "object": {
    "node.name": "speakereq2x2"
  },
  "volume": 1.0
}
```

## Volume Application

- Volumes are applied **once** when the server starts
- Changes to the config file require a server restart to take effect
- For devices: Uses Device Route parameters (hardware volume control)
- For sinks: Uses Node Props parameters (software volume control)
- If an object doesn't support volume control, the setting will be ignored

## Testing

You can verify the volume was applied using:

```bash
# For devices - check via PipeWire CLI (replace 56 with your device ID)
pw-cli enum-params 56 Route | grep channelVolumes

# For sinks - check via PipeWire CLI (replace 81 with your sink/node ID)
pw-cli enum-params 81 Props | grep volume

# Check via REST API (works for both devices and sinks)
curl http://localhost:2716/api/v1/volume
curl http://localhost:2716/api/v1/volume/56
```

To find object IDs:
```bash
pw-cli ls Device  # For hardware devices
pw-cli ls Node    # For sinks and other nodes
```

## See Also

- `link-rules.conf` - Configuration for automatic node linking
- REST API documentation at `/api/v1/devices/:id/volume`
