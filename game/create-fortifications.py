#!/usr/bin/env python3
"""Material-batched masonry walls and fortified drawbridge. Reproducible, no external assets."""
import importlib.util
from pathlib import Path
import struct,json
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('keep',ROOT/'create-keep.py')
k=importlib.util.module_from_spec(spec);spec.loader.exec_module(k)
spec=importlib.util.spec_from_file_location('buildings',ROOT/'import-buildings.py')
b=importlib.util.module_from_spec(spec);spec.loader.exec_module(b)
def generate(item):
    k.geometry[:]=[([],[],[]) for _ in k.materials]
    name=item['id']; k.rng.seed(841+sum(map(ord,name))); folder=ROOT/'assets/models/buildings/fortifications'/item['folder'];folder.mkdir(parents=True,exist_ok=True)
    height={'wooden_wall':4.1,'stone_wall_low':3.3,'stone_wall_high':10.0,'stone_wall_crenellated':10.0,'stone_wall_crenellated_low':3.3,'stone_wall_crenellated_medium':6.663,'stone_wall_crenellated_wood_height':4.1}.get(name,8.0)
    if name=='drawbridge_gate':
        for x in [-3.5,3.5]:
            k.masonry(x,0,2.8,7.5,-1.5,9.5)
            for z in [-3,-1,1,3]:k.box(x,8.5,z,2.8,1,0.8,k.trim)
        k.box(0,7.7,0,4.6,.6,7.5,k.trim)
        k.box(0,.17,0,3.8,.24,6.,k.wood)
        # Outside stairs, clear of the bridge channel.
        for i in range(32):
            h=8*(32-i)/32;z=2.5-(i+.5)*12/32
            k.box(6.4,h/2,z,2.6,h,12/32,k.stone[i%7])
        k.box(4.7,7.9,2.5,3.4,.2,1,k.trim)
        # The moving bridge and portcullis are client components driven by GateState.
        minimum,maximum=[-780,0,-3600],[780,900,3600]
    elif name=='wooden_wall':
        for x in [-.83,.83]:
            for z in [-.75,-.25,.25,.75]:
                k.box(x,2.1,z,.28,4.2,.48,k.wood)
                k.cylinder(x,4.35,z,.22,.5,k.wood,4,top_radius=.01)
        for y in [.8,2.0,3.3]:k.box(0,y,0,1.85,.17,1.9,k.wood)
        minimum,maximum=[-107,0,-107],[107,480,107]
    else:
        k.masonry(0,0,2,2,0,height)
        minimum,maximum=[-107,0,-107],[107,(height+(1.3 if name=='stone_wall_crenellated' else .1))*100,107]
    # Dedicated first primitive is the walkable floor, independent of random stone courses.
    vertices,normals,indices=k.geometry[0];v1,n1,i1=k.geometry[1];offset=len(v1)
    v1.extend(vertices);n1.extend(normals);i1.extend(i+offset for i in indices)
    k.geometry[0]=([],[],[])
    k.box(0,height-.04,0,4.6 if name=='drawbridge_gate' else 1.8,.08,7.5 if name=='drawbridge_gate' else 1.8,k.stone[0])
    # Reuse the glTF serializer but write into this model's folder.
    original=k.ROOT;k.ROOT=folder.parents[4]
    # Serializer targets a fixed folder, so redirect just its output with a local copy.
    source=Path(__file__).with_name('create-keep.py').read_text(encoding='utf-8')
    start=source.index('def write_model():');end=source.index("if __name__=='__main__':",start)
    writer=source[start:end].replace("ROOT/'assets/models/buildings/fortifications/keep'",'output_folder')
    namespace=k.__dict__.copy();namespace['output_folder']=folder
    exec(writer,namespace);namespace['write_model']()
    k.ROOT=original
    (folder/'collision.glb').write_bytes(b.box_collision(minimum,maximum))
    return minimum,maximum
if __name__=='__main__':
    for item in json.loads((ROOT/'building-models.json').read_text(encoding='utf-8')):
        if item.get('procedural')=='fortification':generate(item)
