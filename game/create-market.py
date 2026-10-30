"""A compact Roman forum market with columns, tiled roof and amphora stalls."""
from pathlib import Path
import importlib.util
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('keep',ROOT/'create-keep.py');k=importlib.util.module_from_spec(spec);spec.loader.exec_module(k)
spec=importlib.util.spec_from_file_location('tools',ROOT/'import-buildings.py');t=importlib.util.module_from_spec(spec);spec.loader.exec_module(t)
def generate():
 k.geometry[:]=[([],[],[]) for _ in k.materials];k.rng.seed(1729)
 k.masonry(0,0,6.1,5.7,0,.32)
 for x in [-2.55,2.55]:
  for z in [-2.25,-.75,.75,2.25]:
   k.cylinder(x,1.95,z,.18,3.25,k.trim,12)
   for y in [.42,3.53]:k.box(x,y,z,.52,.22,.52,k.trim)
 k.tiled_roof(0,0,6.4,5.9,3.7,.9)
 for x in [-1.65,1.65]:
  k.box(x,.83,-.6,1.2,1.,3.1,k.wood)
  for z in [-1.55,-.6,.35]:
   k.cylinder(x,1.64,z,.22,.66,k.roof[2],12,top_radius=.15)
   k.box(x+.32,1.47,z,.25,.25,.28,k.leaf)
 folder=ROOT/'assets/models/buildings/supply/market';folder.mkdir(parents=True,exist_ok=True)
 source=(ROOT/'create-keep.py').read_text(encoding='utf8');writer=source[source.index('def write_model():'):source.index("if __name__=='__main__':")].replace("ROOT/'assets/models/buildings/fortifications/keep'",'output_folder');ns=k.__dict__.copy();ns['output_folder']=folder;exec(writer,ns);ns['write_model']()
 minimum,maximum=[-320.,0.,-300.],[320.,470.,300.];(folder/'collision.glb').write_bytes(t.box_collision(minimum,maximum));return minimum,maximum
if __name__=='__main__':generate()
