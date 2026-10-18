"""Compose the unchanged farm mesh with eight reusable fruit-bearing trees.

Called by import-buildings.py; mesh/image buffers are embedded once and shared
by eight static nodes. Collision reserves the entire orchard on the build grid.
"""
import math
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parent


def generate(item, unpack, pack, box_collision):
    source = ROOT.parent / 'assets/models/meshes/buildings'
    doc, tail = unpack((source / 'farm_apple.glb').read_bytes())
    tree, tree_tail = unpack((source / 'apple_tree_apples.glb').read_bytes())
    assert not tree.get('skins') and not tree.get('animations')
    assert len(tree['meshes']) == 1
    binary = bytearray(tail[8:8 + doc['buffers'][0]['byteLength']])
    binary.extend(b'\0' * (-len(binary) % 4))
    offset = len(binary)
    binary.extend(tree_tail[8:8 + tree['buffers'][0]['byteLength']])
    keys = ['bufferViews','accessors','images','samplers','textures','materials','meshes']
    offsets = {key: len(doc.get(key, [])) for key in keys}
    for view in tree['bufferViews']:
        view['buffer'] = 0
        view['byteOffset'] = view.get('byteOffset', 0) + offset
    for accessor in tree['accessors']:
        if 'bufferView' in accessor:
            accessor['bufferView'] += offsets['bufferViews']
    for image in tree.get('images', []):
        image['bufferView'] += offsets['bufferViews']
    for texture in tree.get('textures', []):
        texture['source'] += offsets['images']
        if 'sampler' in texture:
            texture['sampler'] += offsets['samplers']
    def remap_textures(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key.endswith('Texture') and isinstance(child, dict) and 'index' in child:
                    child['index'] += offsets['textures']
                else:
                    remap_textures(child)
        elif isinstance(value, list):
            for child in value:
                remap_textures(child)
    for material in doc['materials']:
        material['alphaMode'] = 'OPAQUE'
    for index, material in enumerate(tree['materials']):
        remap_textures(material)
        material['alphaMode'] = 'MASK' if index == 1 else 'OPAQUE'
        if index == 1:
            material['alphaCutoff'] = 0.4
        material['doubleSided'] = True
    for mesh in tree['meshes']:
        for primitive in mesh['primitives']:
            primitive['attributes'] = {key: index + offsets['accessors'] for key, index in primitive['attributes'].items()}
            primitive['indices'] += offsets['accessors']
            primitive['material'] += offsets['materials']
    positions = [tree['accessors'][p['attributes']['POSITION'] - offsets['accessors']]
                 for mesh in tree['meshes'] for p in mesh['primitives']]
    lo = [min(a['min'][i] for a in positions) for i in range(3)]
    hi = [max(a['max'][i] for a in positions) for i in range(3)]
    center = [(lo[i]+hi[i])/2 for i in range(3)]
    for key in keys:
        doc.setdefault(key, []).extend(tree.get(key, []))
    minimum, maximum = item['collision_bounds']
    cx, cz = (minimum[0]+maximum[0])/2, (minimum[2]+maximum[2])/2
    for index, (x,z) in enumerate([(-5,-5),(0,-5),(5,-5),(-5,0),(5,0),(-5,5),(0,5),(5,5)]):
        angle = (index % 4) * math.pi/2
        c, s = math.cos(angle), math.sin(angle)
        node = len(doc['nodes'])
        doc['nodes'].append({
            'name': f'AppleTree_{index+1}', 'mesh': offsets['meshes'],
            'translation': [cx+x*100-(c*center[0]+s*center[2]), -lo[1], cz+z*100-(-s*center[0]+c*center[2])],
            'rotation': [0,math.sin(angle/2),0,math.cos(angle/2)],
        })
        doc['scenes'][doc.get('scene',0)]['nodes'].append(node)
    doc['buffers'][0]['byteLength'] = len(binary)
    binary.extend(b'\0' * (-len(binary) % 4))
    folder = ROOT / 'assets/models/buildings' / item['category'] / item['folder']
    (folder / 'mesh.glb').write_bytes(pack(doc, struct.pack('<II',len(binary),0x004e4942)+binary))
    (folder / 'collision.glb').write_bytes(box_collision(minimum, maximum))
    return minimum, maximum
