"""Compact Roman river fishery: timber piles, tiled canopy, nets and amphorae."""
import importlib.util,json
from pathlib import Path
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('keep',ROOT/'create-keep.py');k=importlib.util.module_from_spec(spec);spec.loader.exec_module(k)
spec=importlib.util.spec_from_file_location('tools',ROOT/'import-buildings.py');tools=importlib.util.module_from_spec(spec);spec.loader.exec_module(tools)
def generate():
 k.geometry[:]=[([],[],[]) for _ in k.materials];k.rng.seed(431)
 for x in [-1.65,1.65]:
  for z in [-2.1,0.,2.1]:k.cylinder(x,-.6,z,.13,3.,k.wood,8)
 for i in range(18):k.box(0,.55,-2.3+(i+.5)*4.6/18,3.6,.16,4.6/18-.02,k.wood)
 for x in [-1.45,1.45]:
  for z in [-1.7,.8]:k.cylinder(x,1.65,z,.1,2.2,k.wood,8)
 k.tiled_roof(0,-.45,3.4,3.,2.8,.7)
 for x in [-.9,0,.9]:k.cylinder(x,.95,-1.25,.24,.65,k.roof[2],12,top_radius=.16)
 for z in [-1.,-.75,-.5,-.25,0.,.25,.5,.75,1.]:k.box(-1.82,1.15,z,.04,1.,.025,k.trim)
 for y in [.72,.9,1.08,1.26,1.44]:k.box(-1.82,y,0,.03,.024,2.1,k.trim)
 folder=ROOT/'assets/models/buildings/supply/fishpond';folder.mkdir(parents=True,exist_ok=True)
 source=(ROOT/'create-keep.py').read_text(encoding='utf-8');writer=source[source.index('def write_model():'):source.index("if __name__=='__main__':")].replace("ROOT/'assets/models/buildings/fortifications/keep'",'output_folder')
 ns=k.__dict__.copy();ns['output_folder']=folder;exec(writer,ns);ns['write_model']()
 minimum,maximum=[-200.,0.,-250.],[200.,380.,250.];(folder/'collision.glb').write_bytes(tools.box_collision(minimum,maximum));return minimum,maximum
if __name__=='__main__':generate()
