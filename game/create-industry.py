"""Roman production buildings and artillery: reproducible material-batched 3D geometry."""
from pathlib import Path
import importlib.util,json,math,re
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('keep',ROOT/'create-keep.py');k=importlib.util.module_from_spec(spec);spec.loader.exec_module(k)
spec=importlib.util.spec_from_file_location('tools',ROOT/'import-buildings.py');t=importlib.util.module_from_spec(spec);spec.loader.exec_module(t)
def save(folder):
 source=(ROOT/'create-keep.py').read_text(encoding='utf8');start=source.index('def write_model():');end=source.index("if __name__=='__main__':",start)
 writer=source[start:end].replace("ROOT/'assets/models/buildings/fortifications/keep'",'output_folder');ns=k.__dict__.copy();ns['output_folder']=folder;exec(writer,ns);ns['write_model']()

def generate_wheat_patch():
 reset();grain=k.material('Ripe wheat',(.76,.57,.19));leaf=k.material('Wheat leaves',(.30,.39,.10))
 for i in range(16):
  for j in range(10):
   x=-4.4+i*.57;z=-3.4+j*.60;h=.9+((i*3+j*7)%5)*.05;k.cylinder(x,h/2,z,.018,h,leaf,5);k.cylinder(x,h+.07,z,.055,.23,grain,6,top_radius=.015);k.box(x+.06,h*.5,z,.17,.018,.03,leaf)
 folder=ROOT/'assets/models/industry/wheat_patch';folder.mkdir(parents=True,exist_ok=True);save(folder)

def reset():k.geometry[:]=[([],[],[]) for _ in k.materials];k.rng.seed(1707)
def shelter(x,z,w,d):
 for xx in [x-w/2+.2,x+w/2-.2]:
  for zz in [z-d/2+.2,z+d/2-.2]:k.cylinder(xx,1.7,zz,.18,3.4,k.trim)
 k.tiled_roof(x,z,w,d,3.5,.8)
def fence(x0,x1,z0,z1):
 for x in [x0,x1]:
  for z in [z0,z1]:k.box(x,.8,z,.16,1.6,.16,k.wood)
 for y in [.5,1.1]:
  for x in [x0,x1]:k.box(x,y,(z0+z1)/2,.13,.13,z1-z0,k.wood)
  for z in [z0,z1]:k.box((x0+x1)/2,y,z,x1-x0,.13,.13,k.wood)
def artillery(kind,x=0,z=0,scale=1):
 def box(a,y,b,w,h,d,m):k.box(x+a*scale,y*scale,z+b*scale,w*scale,h*scale,d*scale,m)
 for a in [-.65,.65]:box(a,.5,0,.16,.22,2.4,k.wood)
 for b in [-.8,.8]:box(0,.5,b,1.6,.16,.16,k.wood)
 for a in [-.8,.8]:
  for b in [-.8,.8]:
   # Thick octagonal wheel, with visible rim and bronze hub.
   for i in range(12):
    theta=2*math.pi*i/12;k.box(x+a*scale,(.5+.35*math.sin(theta))*scale,z+(b+.35*math.cos(theta))*scale,.16*scale,.16*scale,.16*scale,k.wood)
   box(a,.5,b,.22,.18,.18,k.gold)
 if kind=='ballista':
  box(0,1.0,.15,.18,.2,2.5,k.wood);box(0,1.0,.5,2.7,.18,.16,k.wood)
  for a in [-1.,1.]:box(a,1.0,.5,.2,.7,.35,k.gold)
  box(0,1.1,1.,.05,.05,2.0,k.black)
 else:
  for a in [-.5,.5]:box(a,1.1,0,.2,1.5,.2,k.wood)
  box(0,1.6,-.2,.22,2.1,.22,k.wood);k.cylinder(x,2.6*scale,z-.2*scale,.32*scale,.16*scale,k.black)
  box(0,.7,.8,1.3,.2,.8,k.roof[2])
def generate(item):
 reset();name=item['id'];w,d=[v*2 for v in item['grid']];k.box(0,.06,0,w-.1,.12,d-.1,k.stone[2])
 if name=='wheat_farm':
  k.box(0,.13,-.5,10,.12,7,k.mortar)
  for z in range(-3,4):k.box(0,.22,z-.6,9.8,.14,.12,k.wood)
  shelter(-3.8,3.3,3.8,2.5);k.cylinder(-3.8,.6,3.3,.45,.9,k.roof[3])
 elif name=='windmill':
  shelter(0,-.7,7.5,5.8)
  for x in [-2.,2.]:
   k.cylinder(x,.4,0,.9,.5,k.stone[0],20);k.cylinder(x,.95,0,.65,.6,k.stone[4],20,top_radius=.3);k.box(x,1.25,0,2.4,.15,.15,k.wood)
  for z in [2.,2.7]:k.cylinder(-2.,.55,z,.3,.8,k.roof[3]);k.cylinder(2.,.55,z,.3,.8,k.roof[2])
 elif name=='bakery':
  shelter(0,-1.,7.6,5.5)
  for x in [-2.,2.]:
   k.cylinder(x,.5,-1.,1.25,.8,k.stone[2],20);k.cylinder(x,1.35,-1.,1.2,1.25,k.roof[2],20,top_radius=.3);k.box(x,.85,.22,.65,.8,.08,k.black);k.cylinder(x,2.6,-1.,.22,1.4,k.stone[3])
  k.box(0,.85,2.2,5.5,.16,1.,k.wood)
  for x in [-2.,-1.,0.,1.,2.]:k.cylinder(x,1.02,2.2,.22,.18,k.roof[4],12)
 elif name=='stable':
  shelter(-3.,0.,5.6,9.4);k.masonry(-5.6,0,.3,9.,0.,2.8);fence(.2,5.7,-4.6,4.6)
  for z in [-3.,0.,3.]:k.box(-3.,.7,z,4.8,.12,.12,k.wood)
  k.box(4.6,.35,-3.5,1.5,.6,.65,k.trim)
 elif name=='siege_workshop':
  shelter(0,-1.5,9.5,4.6);k.masonry(0,-3.6,9.,.3,0.,3.)
  k.box(-2.,.9,1.8,3.,.25,1.4,k.wood);artillery('ballista',2.,1.7,.8)
  for z in [1.,2.,3.]:k.box(-3.4,.45,z,1.6,.22,.22,k.wood)
 elif name=='harbor':
  # Local +Z projects into the river, local -Z stays on the bank.
  shelter(0,-3.,7.5,3.5)
  for x in [-3.5,0.,3.5]:
   for z in [0.,2.4,4.6]:k.cylinder(x,-.5,z,.15,1.8,k.wood)
  for z in [i*.2 for i in range(25)]:k.box(0,.35,z,7.6,.18,.18,k.wood)
  k.box(-2.,1.9,.2,.24,3.1,.24,k.wood);k.box(-.6,3.2,.2,3.,.22,.22,k.wood)
  for x,z in [(2.5,-2.5),(-2.5,-2.5),(2.7,1.)]:k.box(x,.7,z,.7,.7,.7,k.wood)
 else:raise ValueError(name)
 folder=ROOT/'assets/models/buildings'/item['category']/item['folder'];folder.mkdir(parents=True,exist_ok=True);save(folder)
 lo=[-w*50.,0.,-d*50.];hi=[w*50.,500. if name=='siege_workshop' else 480.,d*50.];(folder/'collision.glb').write_bytes(t.box_collision(lo,hi));return lo,hi
if __name__=='__main__':
 catalog=json.loads((ROOT/'building-models.json').read_text(encoding='utf8'));bounds={}
 for item in catalog:
  if item.get('procedural')=='roman_industry':bounds[item['id']]=generate(item)
 for name in ['ballista','onager']:
  reset();artillery(name);folder=ROOT/'assets/models/units'/name;folder.mkdir(parents=True,exist_ok=True);save(folder)
 generate_wheat_patch()
 # Keep the static catalog in agreement with these authored collision bounds.
 p=ROOT.parent/'shared/src/building_catalog.rs';s=p.read_text(encoding='utf8')
 for name,(lo,hi) in bounds.items():
  pat=r'(    Definition \{\s*name: "'+name+r'",.*?\},)';m=re.search(pat,s,re.S);assert m,name;block=re.sub(r'min: \[.*?\],','min: '+repr(lo)+',',m.group());block=re.sub(r'max: \[.*?\],','max: '+repr(hi)+',',block);s=s[:m.start()]+block+s[m.end():]
 p.write_text(s,encoding='utf8',newline='\n')
