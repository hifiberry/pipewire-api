# Device Volumes Configuration

Configuration file for setting default volumes on PipeWire devices at server startup.

## Configuration File Locations

The server looks for device volume configuration in the following locations (in order of precedence):

1. **User config**: `~/.config/pipewire-api/device-volumes.conf` (highest priority)
2. **System config**: `/etc/pipewire-api/device-volumes.conf` (fallback)

## Configuration Format

The configuration file is a JSON array of device volume rules:

```json
[
  {
    "name": "HiFiBerry DAC Default Volume",
    "device": {
      "device.name": "alsa_card\\.platform-soc_.*_sound"
    },
    "volume": 0.75
  }
]
```

### Rule Properties

- **name**: Human-readable description of the rule
- **device**: Object containing device matching criteria (key-value pairs)
  - Keys are PipeWire device property names
  - Values are regex patterns to match against
  - All specified properties must match for the rule to apply
- **volume**: Volume level to set (0.0 = mute, 1.0 = full volume, linear scale)

## Device Matching

Device properties are matched using regular expressions. Common properties include:

- `device.name`: Device name (e.g., `alsa_card.usb-Audio_Device`)
- `device.description`: Human-readable description
- `device.api`: API used (e.g., `alsa`, `v4l2`)
- `media.class`: Media class (e.g., `Audio/Device`)

Use `pw-dump` or `pw-cli ls Device` to see available devices and their properties.

## Examples

### Set volume for all ALSA devices:
```json
{
  "name": "All ALSA Devices",
  "device": {
    "device.api": "alsa"
  },
  "volume": 0.8
}
```

### Set volume for USB audio devices:
```json
{
  "name": "USB Audio Devices",
  "device": {
    "device.name": "alsa_card\\.usb-.*"
  },
  "volume": 0.9
}
```

### Set volume for specific device:
```json
{
  "name": "HiFiBerry DAC+ ADC",
  "device": {
    "device.name": "alsa_card\\.platform-soc_.*_sound",
    "device.nick": "snd_rpi_hifiberry_dacplusadc"
  },
  "volume": 0.75
}
```

## Volume Application

- Volumes are applied **once** when the server starts
- Changes to the config file require a server restart to take effect
- Volumes are set via Device Route parameters (hardware volume control)
- If a device doesn't support volume control, the setting will be ignored

## Testing

You can verify the volume was applied using:

```bash
# Check via PipeWire CLI (replace 56 with your device ID)
pw-cli enum-params 56 Route | grep channelVolumes

# Check via REST API
curl http://localhost:2716/api/v1/devices/56/volume
```

To find your device ID:
```bash
pw-cli ls Device
```

## See Also

- `link-rules.conf` - Configuration for automatic node linking
- REST API documentation at `/api/v1/devices/:id/volume`
