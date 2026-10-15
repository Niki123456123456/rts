#!/usr/bin/env python3
"""Package the two supplied GLBs without requiring the ignored asset archive in CI.

The converted DDS alpha channels carry surface data, not transparency. The source
export marks every material BLEND, making walls and roofs translucent. Use OPAQUE
for this closed granary; geometry, collision and embedded PNG bytes stay intact.
"""
import json
from pathlib import Path
import struct

root = Path(__file__).resolve().parent
source = root.parent / 'assets/models/meshes/buildings'
destination = root / 'assets/models/buildings'
destination.mkdir(parents=True, exist_ok=True)
for name in ('granary_closed', 'granary_collision'):
    data = (source / f'{name}.glb').read_bytes()
    if name == 'granary_closed':
        length, kind = struct.unpack_from('<II', data, 12)
        document = json.loads(data[20:20 + length])
        for material in document['materials']:
            material['alphaMode'] = 'OPAQUE'
        encoded = json.dumps(document, separators=(',', ':')).encode()
        encoded += b' ' * (-len(encoded) % 4)
        tail = data[20 + length:]
        data = struct.pack('<III', 0x46546c67, 2, 20 + len(encoded) + len(tail)) + struct.pack('<II', len(encoded), kind) + encoded + tail
    (destination / f'{name}.glb').write_bytes(data)
