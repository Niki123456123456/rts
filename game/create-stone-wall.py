#!/usr/bin/env python3
"""Generate the 2 m stone wall using the original archive's masonry textures."""
import importlib.util
import sys
sys.dont_write_bytecode = True
import json
from pathlib import Path
import struct
ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('import_buildings', ROOT / 'import-buildings.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

def generate():
    # Source model units are centimetres. Roof surface matches the stair tower.
    lo, hi = [-100., 0., -100.], [100., 666.3, 100.]
    faces = [
        ([(-100,0,100),(100,0,100),(100,666.3,100),(-100,666.3,100)], (0,0,1), 0),
        ([(100,0,-100),(-100,0,-100),(-100,666.3,-100),(100,666.3,-100)], (0,0,-1), 0),
        ([(100,0,100),(100,0,-100),(100,666.3,-100),(100,666.3,100)], (1,0,0), 0),
        ([(-100,0,-100),(-100,0,100),(-100,666.3,100),(-100,666.3,-100)], (-1,0,0), 0),
        ([(-100,666.3,100),(100,666.3,100),(100,666.3,-100),(-100,666.3,-100)], (0,1,0), 1),
        ([(-100,0,-100),(100,0,-100),(100,0,100),(-100,0,100)], (0,-1,0), 1),
    ]
    binary = bytearray()
    views, accessors, primitives = [], [], []
    def view(data, target=None):
        binary.extend(b'\0' * (-len(binary) % 4))
        result = len(views)
        views.append(dict(buffer=0, byteOffset=len(binary), byteLength=len(data), **({'target':target} if target else {})))
        binary.extend(data)
        return result
    def accessor(values, shape, component=5126, bounds=None):
        count = len(values)
        data = b''.join(struct.pack('<' + ('f' if component == 5126 else 'H') * len(v), *v) for v in values)
        result = len(accessors)
        accessors.append(dict(bufferView=view(data, 34963 if component==5123 else 34962), componentType=component, count=count, type=shape, **(bounds or {})))
        return result
    for points, normal, material in faces:
        pos = accessor(points, 'VEC3', bounds={'min':[min(p[i] for p in points) for i in range(3)],'max':[max(p[i] for p in points) for i in range(3)]})
        norm = accessor([normal]*4, 'VEC3')
        uv = accessor([(0,0),(1,0),(1,1),(0,1)], 'VEC2')
        idx = accessor([(i,) for i in [0,1,2,0,2,3]], 'SCALAR', 5123)
        primitives.append(dict(attributes={'POSITION':pos,'NORMAL':norm,'TEXCOORD_0':uv},indices=idx,material=material))
    images = [{'bufferView':view((ROOT.parent / 'assets/images/Source/meshes/buildings' / filename).read_bytes()),'mimeType':'image/png'} for filename in ['castle_wall.png','walltop.png']]
    binary.extend(b'\0' * (-len(binary) % 4))
    doc = dict(asset={'version':'2.0','generator':'Stronghold stone-wall generator'},scene=0,scenes=[{'nodes':[0]}],nodes=[{'mesh':0}],meshes=[{'primitives':primitives}],buffers=[{'byteLength':len(binary)}],bufferViews=views,accessors=accessors,images=images,samplers=[{'wrapS':10497,'wrapT':10497}],textures=[{'source':i,'sampler':0} for i in range(2)],materials=[{'name':name,'alphaMode':'OPAQUE','pbrMetallicRoughness':{'baseColorTexture':{'index':i},'metallicFactor':0,'roughnessFactor':1}} for i,name in enumerate(['Castle masonry','Stone walkway'])])
    folder = ROOT / 'assets/models/buildings/fortifications/stone_wall'
    folder.mkdir(parents=True, exist_ok=True)
    (folder/'mesh.glb').write_bytes(module.pack(doc, struct.pack('<II',len(binary),0x004e4942)+binary))
    (folder/'collision.glb').write_bytes(module.box_collision(lo,hi))
    return lo, hi

if __name__=='__main__': generate()
