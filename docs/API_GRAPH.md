# Graph API

The Graph API provides visual representations of the PipeWire audio topology.

## Base URL

All endpoints are prefixed with `/api/v1`

## Endpoints

### GET /graph

Returns a DOT format graph of the audio topology.

**Response:**
- Content-Type: `text/vnd.graphviz`
- Body: DOT format graph

**Example:**
```bash
curl http://localhost:2716/api/v1/graph
```

**Response:**
```dot
digraph PipeWire {
    rankdir=LR;
    node [shape=box, style=filled];
    
    subgraph cluster_devices {
        label="Devices";
        style=dashed;
        color=gray;
        dev_56 [label="alsa_card.platform-soc_107c000000_sound", fillcolor=lightgray];
    }

    // Audio Nodes
    node_31 [label="Dummy-Driver\nID: 31", fillcolor=white];
    node_38 [label="effect_input.proc\nID: 38", fillcolor=lightblue];
    node_41 [label="riaa\nID: 41", fillcolor=lightgreen];
    node_81 [label="alsa_output...\nID: 81", fillcolor=lightblue];

    // Links
    node_39 -> node_44;
    node_45 -> node_81;

    // Legend
    subgraph cluster_legend {
        label="Legend";
        legend_sink [label="Sink/Playback", fillcolor=lightblue];
        legend_source [label="Source/Capture", fillcolor=lightgreen];
        legend_filter [label="Filter", fillcolor=lightyellow];
    }
}
```

### GET /graph/png

Returns a PNG image of the audio topology graph. Requires `graphviz` to be installed on the system.

**Response:**
- Content-Type: `image/png`
- Body: PNG image data

**Errors:**
- 404 Not Found: If graphviz is not installed

**Example:**
```bash
curl -o graph.png http://localhost:2716/api/v1/graph/png
```

## Node Colors

The graph uses color coding to distinguish different node types:

| Color | Node Type |
|-------|-----------|
| Light Blue | Sink / Playback |
| Light Green | Source / Capture |
| Light Yellow | Filter |
| White | Other audio nodes |
| Light Gray | Devices |

## Notes

- Only audio nodes are shown (MIDI and video nodes are filtered out)
- Devices are shown in a separate cluster
- Links represent audio connections between nodes (aggregated from port-level links)
- The graph uses left-to-right layout (`rankdir=LR`)
