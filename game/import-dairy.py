#!/usr/bin/env python3
"""Package the original cow rig with in-place clips and a textured paddock fence."""
import importlib.util
import json
import math
from pathlib import Path
import struct
import sys
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('buildings', ROOT / 'import-buildings.py')
buildings = importlib.util.module_from_spec(spec)
spec.loader.exec_module(buildings)
OUTPUT = ROOT / 'assets/models/animals/dairy'
OUTPUT.mkdir(parents=True, exist_ok=True)


def mul(a, b):
    x,y,z,w = a
    X,Y,Z,W = b
    return [w*X+x*W+y*Z-z*Y,w*Y-x*Z+y*W+z*X,w*Z+x*Y-y*X+z*W,w*W-x*X-y*Y-z*Z]


def inv(q):
    return [-q[0],-q[1],-q[2],q[3]]


def axis(v, angle):
    return [*(x*math.sin(angle/2) for x in v), math.cos(angle/2)]


def cow():
    source = ROOT.parent / 'assets/models/meshes/units/animals/cow.glb'
    doc, tail = buildings.unpack(source.read_bytes())
    binary = bytearray(tail[8:8 + doc['buffers'][0]['byteLength']])
    joints = {i for skin in doc['skins'] for i in skin['joints']}
    names = {doc['nodes'][i]['name']: i for i in joints}
    world = {}
    def visit(i, rotation):
        world[i] = mul(rotation, doc['nodes'][i].get('rotation', [0,0,0,1]))
        for child in doc['nodes'][i].get('children', []):
            visit(child, world[i])
    for i in doc['scenes'][0]['nodes']:
        visit(i, [0,0,0,1])

    def accessor(values, kind):
        binary.extend(b'\0' * (-len(binary) % 4))
        start = len(binary)
        flat = [x for value in values for x in (value if isinstance(value,list) else [value])]
        binary.extend(struct.pack('<'+'f'*len(flat), *flat))
        view = len(doc['bufferViews'])
        doc['bufferViews'].append({'buffer':0,'byteOffset':start,'byteLength':len(binary)-start})
        entry = {'bufferView':view,'componentType':5126,'count':len(values),'type':kind}
        if kind == 'SCALAR':
            entry.update(min=[min(values)], max=[max(values)])
        index = len(doc['accessors'])
        doc['accessors'].append(entry)
        return index

    doc['animations'] = []
    for state, duration in [('idle',3.6), ('walking',1.4)]:
        frames = 41
        times = [duration * i/(frames-1) for i in range(frames)]
        input_ = accessor(times, 'SCALAR')
        animation = {'name':state,'channels':[],'samplers':[]}
        motions = [('Bip01 Head', .035, 0), ('Bip01 Neck', .025, 0)]
        if state == 'walking':
            motions += [(f'Bip01 {side} {part}', amplitude, phase)
                        for side, phase in [('L',0),('R',math.pi)]
                        for part, amplitude in [('UpperArm',.22),('Forearm',.14),('Thigh',-.22),('Calf',-.14)]]
        for name, amplitude, phase in motions:
            if name not in names:
                continue
            node = names[name]
            rest = doc['nodes'][node].get('rotation', [0,0,0,1])
            # Bend around world X in the existing bone's local coordinate system.
            local_axis = mul(mul(inv(world[node]), [1,0,0,0]), world[node])[:3]
            rotations = [mul(rest, axis(local_axis, amplitude*math.sin(2*math.pi*i/(frames-1)+phase))) for i in range(frames)]
            output = accessor(rotations, 'VEC4')
            index = len(animation['samplers'])
            animation['samplers'].append({'input':input_,'output':output,'interpolation':'LINEAR'})
            animation['channels'].append({'sampler':index,'target':{'node':node,'path':'rotation'}})
        doc['animations'].append(animation)
    for material in doc.get('materials', []):
        material['alphaMode'] = 'OPAQUE'
    doc['buffers'][0]['byteLength'] = len(binary)
    binary.extend(b'\0' * (-len(binary) % 4))
    (OUTPUT / 'cow.glb').write_bytes(buildings.pack(doc, struct.pack('<II',len(binary),0x004e4942)+binary))


def fence():
    positions, normals, uvs, indices = [], [], [], []
    def box(center, size):
        lo = [c-s/2 for c,s in zip(center,size)]
        hi = [c+s/2 for c,s in zip(center,size)]
        for axis_ in range(3):
            a,b = (axis_+1)%3, (axis_+2)%3
            for side in [0,1]:
                start = len(positions)
                for u,v in [(0,0),(1,0),(1,1),(0,1)]:
                    p = list(lo)
                    p[axis_] = hi[axis_] if side else lo[axis_]
                    p[a] = hi[a] if u else lo[a]
                    p[b] = hi[b] if v else lo[b]
                    normal = [0,0,0]
                    normal[axis_] = 1 if side else -1
                    positions.append(p); normals.append(normal); uvs.append([u,v])
                order = [0,1,2,0,2,3] if side else [0,2,1,0,3,2]
                indices.extend(start+i for i in order)
    # Gate gap faces the dairy buildings; cows remain inside the pasture bounds.
    edges = [((-7,.5),(-1.2,.5)),((1.2,.5),(7,.5)),((-7,7.2),(7,7.2)),((-7,.5),(-7,7.2)),((7,.5),(7,7.2))]
    posts = set()
    for (x,z),(X,Z) in edges:
        count = math.ceil(math.hypot(X-x,Z-z)/2)
        for i in range(count+1):
            posts.add((round(x+(X-x)*i/count,6),round(z+(Z-z)*i/count,6)))
        for y in [.45,1.05]:
            box([(x+X)/2,y,(z+Z)/2], [abs(X-x) or .11,.13,abs(Z-z) or .11])
    for x,z in sorted(posts):
        box([x,.72,z],[.16,1.44,.16])
    binary = bytearray()
    views, accessors = [], []
    def accessor(values, components, type_, fmt, component_type):
        binary.extend(b'\0' * (-len(binary)%4)); start = len(binary)
        flat = [n for value in values for n in (value if isinstance(value,list) else [value])]
        binary.extend(struct.pack('<'+fmt*len(flat), *flat))
        views.append({'buffer':0,'byteOffset':start,'byteLength':len(binary)-start})
        entry = {'bufferView':len(views)-1,'componentType':component_type,'count':len(values),'type':type_}
        if type_ == 'VEC3' and components == 'position':
            entry.update(min=[min(p[i] for p in values) for i in range(3)],max=[max(p[i] for p in values) for i in range(3)])
        accessors.append(entry)
        return len(accessors)-1
    p = accessor(positions,'position','VEC3','f',5126)
    n = accessor(normals,'normal','VEC3','f',5126)
    uv = accessor(uvs,'uv','VEC2','f',5126)
    ix = accessor(indices,'index','SCALAR','I',5125)
    doc = {'asset':{'version':'2.0','generator':'Stronghold dairy paddock'},'scene':0,'scenes':[{'nodes':[0]}],
           'nodes':[{'mesh':0}],'meshes':[{'primitives':[{'attributes':{'POSITION':p,'NORMAL':n,'TEXCOORD_0':uv},'indices':ix,'material':0}]}],
           'materials':[{'name':'Mossy timber','alphaMode':'OPAQUE','pbrMetallicRoughness':{}}],
           'buffers':[{'byteLength':len(binary)}],'bufferViews':views,'accessors':accessors}
    binary.extend(b'\0' * (-len(binary)%4))
    tail = buildings.embed_material_textures(doc, struct.pack('<II',len(binary),0x004e4942)+binary, {'0':'beams_mossy.png'})
    (OUTPUT / 'fence.glb').write_bytes(buildings.pack(doc,tail))


if __name__ == '__main__':
    cow()
    fence()
