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
def beam(a,b,width,mat):
 # A rectangular beam aligned between arbitrary endpoints, retaining wood grain UVs.
 direction=[b[i]-a[i] for i in range(3)];length=math.sqrt(sum(v*v for v in direction));up=[v/length for v in direction]
 def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
 side=cross(up,[0,1,0]);n=math.sqrt(sum(v*v for v in side));side=[v/n for v in side] if n>.01 else [1.,0.,0.];front=cross(side,up);center=[(a[i]+b[i])/2 for i in range(3)]
 start=len(k.geometry[mat][0]);k.box(0,0,0,width,length,width,mat);vertices,normals,_=k.geometry[mat]
 for i in range(start,len(vertices)):
  q=vertices[i];vertices[i]=tuple(center[j]*100+side[j]*q[0]+up[j]*q[1]+front[j]*q[2] for j in range(3))
  q=normals[i];normals[i]=tuple(side[j]*q[0]+up[j]*q[1]+front[j]*q[2] for j in range(3))

def artillery(kind,x=0,z=0,scale=1):
 def b(a,y,c,w,h,d,m):k.box(x+a*scale,y*scale,z+c*scale,w*scale,h*scale,d*scale,m)
 def rod(a,c,width,mat):beam((x+a[0]*scale,a[1]*scale,z+a[2]*scale),(x+c[0]*scale,c[1]*scale,z+c[2]*scale),width*scale,mat)
 iron=k.material('Forged iron',(.23,.24,.23),.75);rope=k.material('Twisted hemp',(.62,.49,.28));hide=k.material('Stitched rawhide',(.47,.37,.24));oak=k.wood
 tower=kind=='siege_tower';ladder=kind=='assault_ladder';ram=kind in ['ram','covered_ram'];screen=kind=='pluteus';scorpion=kind=='scorpion'
 width=2.8 if tower else 2.3 if ram else 1.9;length=3.4 if tower or ram else 2.5
 def wheels():
  for a in [-width/2,width/2]:
   for c in [-length/2+.2,length/2-.2]:
    for i in range(16):
     theta=2*math.pi*i/16;b(a,.48+.4*math.sin(theta),c+.4*math.cos(theta),.16,.16,.16,oak)
    for i in range(8):
     theta=math.pi*i/4;rod((a,.48,c),(a,.48+.34*math.sin(theta),c+.34*math.cos(theta)),.065,oak)
    b(a,.48,c,.22,.2,.2,iron)
 if ladder:
  for a in [-.55,.55]:b(a,.14,0,.12,.16,8.,oak)
  for i in range(18):b(0,.17,-3.7+i*.43,1.2,.12,.12,oak)
  for c in [-3.5,-1.,1.,3.5]:
   for a in [-.55,.55]:b(a,.17,c,.19,.19,.08,rope)
  return
 wheels()
 for a in [-width/2+.18,width/2-.18]:b(a,.48,0,.22,.26,length+.3,oak)
 for c in [-length/2+.2,length/2-.2]:b(0,.48,c,width,.22,.22,oak)
 if tower:
  for y in [.7,3.7,6.7,9.7]:
   for c in [i*.24-length/2 for i in range(15)]:b(0,y,c,width,.12,.22,oak)
  for a in [-1.25,1.25]:
   for c in [-1.5,1.5]:b(a,5.4,c,.22,9.8,.22,oak)
   for y in [2.2,5.2,8.2]:
    for c in [i*.25-1.4 for i in range(12)]:b(a,y,c,.13,2.8,.23,oak)
    rod((a,y-1.3,-1.4),(a,y+1.3,1.4),.16,oak)
    for c in [-1.3,0,1.3]:b(a,y,c,.18,2.5,.035,hide)
   for y in [.9,3.9,6.9,9.9]:b(a,y,0,.28,.12,3.4,iron)
  for a in [i*.3-1.2 for i in range(9)]:b(a,9.7,2.3,.28,.15,2.,oak)
  for a in [-1.2,1.2]:
   for c in [-1.2,-.4,.4,1.2]:b(a,10.3,c,.22,.9,.4,oak)
  for y in [1.,4.,7.]:
   for a in [-.7,.7]:rod((a,y,-1.2),(a,y+2.5,1.2),.1,oak)
   for i in range(8):b(0,y+i*.32,-1.2+i*.3,1.5,.1,.1,oak)
 elif ram:
  for c in [-1.1,1.1]:
   for a in [-1.,1.]:rod((a,.5,c),(0,2.7,c),.22,oak)
  b(0,2.7,0,.26,.24,3.5,oak)
  # Faceted hanging ram, bronze/iron head, suspension ropes and straps.
  for i in range(16):
   theta=math.pi*2*i/16;b(.24*math.cos(theta),1.4+.24*math.sin(theta),.25,.15,.15,4.3,oak)
  for c in [-1.,1.]:rod((0,2.6,c),(0,1.4,c),.08,rope)
  for c in [2.1,2.3]:b(0,1.4,c,.58,.58,.18,iron)
  if kind=='covered_ram':
   for a in [-1.1,1.1]:
    for c in [-1.4,1.4]:b(a,1.5,c,.17,2.,.17,oak)
    for c in [i*.36-1.5 for i in range(9)]:b(a,1.75,c,.08,2.1,.34,hide)
   for a in [i*.24-1.2 for i in range(11)]:b(a,2.85-abs(a)*.35,0,.23,.12,3.5,hide)
 elif screen:
  for a in [-1.1,1.1]:b(a,1.65,.75,.18,2.4,.18,oak);rod((a,.6,-.8),(a,2.5,.75),.14,oak)
  for a in [i*.22-1.1 for i in range(11)]:
   # Actual gaps remain as shooting ports in the hide and wicker face.
   for y,h in [(.95,.65),(2.15,.9)]:b(a,y,.8,.21,h,.12,hide)
   if abs(a)>.3:b(a,1.5,.8,.21,.55,.12,hide)
  for a in [-1.15,1.15]:b(a,1.65,.83,.07,2.45,.06,rope)
 else:
  if scorpion:
   for a,c in [(-.7,-.5),(.7,-.5),(0,.8)]:rod((a,.55,c),(0,1.5,0),.17,oak)
  y=1.75 if scorpion else 1.35
  for a in [-.8,.8]:
   b(a,y,.3,.23,1.3,.23,oak)
   for yy in [i*.065+y-.37 for i in range(12)]:b(a,yy,.3,.32,.055,.36,rope)
   for yy in [y-.65,y+.65]:b(a,yy,.3,.4,.18,.5,iron)
  if kind=='onager':
   b(0,.75,-.75,1.8,.45,.75,hide);rod((0,1.,0),(0,3.5,1.5),.2,oak);b(0,3.5,1.5,.5,.18,.5,hide)
   for a in [-.9,.9]:b(a,1.5,0,.25,1.8,.25,oak)
   b(0,1.,0,2.,.32,.32,rope)
  else:
   b(0,y+.1,0,.2,.18,3.1,oak)
   for a in [-1.,1.]:rod((a*.8,y,.3),(a*1.8,y,.95),.12,oak);rod((a*1.8,y,.95),(0,y,-.5),.035,rope)
   for a in [-.8,.8]:rod((a,.5,-1.),(a,y+.6,.3),.18,oak)
   if kind=='stone_ballista':k.cylinder(x,y*scale+.22*scale,z+.15*scale,.27*scale,.45*scale,k.stone[1],12)
   else:rod((0,y+.25,-.6),(0,y+.25,2.),.04,iron)
 # Metal straps and individual rivets break up the timber silhouettes.
 for a in [-width/2+.18,width/2-.18]:
  for c in [-length/2+.2,length/2-.2]:
   b(a,.62,c,.32,.05,.35,iron)
   for dx in [-.1,.1]:b(a+dx,.66,c,.045,.045,.045,iron)

def save_engine(folder):
 # Embed repeatable wood-grain and hide textures; geometry uses world-scale UVs.
 import struct,random,zlib
 binary=bytearray();views=[];accessors=[];primitives=[];materials=json.loads(json.dumps(k.materials));rng=random.Random(912)
 def acc(values,shape,integer=False):
  flat=[v for p in values for v in (p if isinstance(p,(tuple,list)) else [p])];raw=struct.pack('<'+('I' if integer else 'f')*len(flat),*flat);binary.extend(b'\0'*(-len(binary)%4));offset=len(binary);binary.extend(raw);views.append(dict(buffer=0,byteOffset=offset,byteLength=len(raw)));rec=dict(bufferView=len(views)-1,componentType=5125 if integer else 5126,count=len(values),type=shape)
  if shape=='VEC3':rec.update(min=[min(p[i] for p in values) for i in range(3)],max=[max(p[i] for p in values) for i in range(3)])
  accessors.append(rec);return len(accessors)-1
 for mat,(vertices,normals,indices) in enumerate(k.geometry):
  if not indices:continue
  uv=[(p[2]/100 if abs(n[0])>.5 else p[0]/100,p[2]/100 if abs(n[1])>.5 else p[1]/100) for p,n in zip(vertices,normals)]
  primitives.append(dict(attributes=dict(POSITION=acc(vertices,'VEC3'),NORMAL=acc(normals,'VEC3'),TEXCOORD_0=acc(uv,'VEC2')),indices=acc(indices,'SCALAR',True),material=mat))
 images=[]
 for hide in [False,True]:
  pixels=[]
  for y in range(128):
   for x in range(128):
    grain=math.sin(x*.7+math.sin(y*.08)*2)*.08+math.sin(x*.19+y*.012)*.04+rng.uniform(-.04,.04) if not hide else rng.uniform(-.08,.08)+math.sin(x*.15)*math.sin(y*.11)*.04
    pixels.append(tuple(int(max(0,min(255,channel*(1+grain)))) for channel in ([210,193,165] if hide else [227,204,172])))
  def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
  scan=b''.join(b'\0'+bytes(v for p in pixels[y*128:(y+1)*128] for v in p) for y in range(128))
  raw=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',128,128,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(scan))+chunk(b'IEND',b'');binary.extend(b'\0'*(-len(binary)%4));offset=len(binary);binary.extend(raw);views.append(dict(buffer=0,byteOffset=offset,byteLength=len(raw)));images.append(dict(bufferView=len(views)-1,mimeType='image/png'))
 for mat in materials:
  if mat['name']=='Dark oak' or 'rawhide' in mat['name']:mat['pbrMetallicRoughness']['baseColorTexture']={'index':1 if 'rawhide' in mat['name'] else 0}
 doc=dict(asset={'version':'2.0','generator':'RTS Roman siege workshop'},scene=0,scenes=[{'nodes':[0]}],nodes=[{'mesh':0}],meshes=[{'primitives':primitives}],buffers=[{'byteLength':len(binary)}],bufferViews=views,accessors=accessors,materials=materials,images=images,textures=[{'sampler':0,'source':0},{'sampler':0,'source':1}],samplers=[{'wrapS':10497,'wrapT':10497,'magFilter':9729,'minFilter':9987}])
 raw=json.dumps(doc,separators=(',',':')).encode();raw+=b' '*(-len(raw)%4);binary+=b'\0'*(-len(binary)%4);folder.mkdir(parents=True,exist_ok=True);(folder/'mesh.glb').write_bytes(struct.pack('<III',0x46546c67,2,28+len(raw)+len(binary))+struct.pack('<II',len(raw),0x4e4f534a)+raw+struct.pack('<II',len(binary),0x004e4942)+binary)

def generate(item):
 reset();name=item['id'];w,d=[v*2 for v in item['grid']];k.box(0,.06,0,w-.1,.12,d-.1,k.stone[2])
 if name=='barracks':
  for x in [-4.4,4.4]:shelter(x,0.,3.,8.5)
  k.masonry(0,-4.5,11.5,.35,0.,3.2)
  for x in [-3.,0.,3.]:
   k.box(x,.8,-3.5,2.,.15,.5,k.wood)
   for xx in [-.6,0.,.6]:k.box(x+xx,1.25,-3.5,.08,1.3,.08,k.gold)
  for x in [-4.,4.]:k.cylinder(x,2.,4.2,.17,3.8,k.trim)
 elif name in ['dirt_road','stone_road','roman_bridge']:
  k.geometry[:]=[([],[],[]) for _ in k.materials]
  if name=='dirt_road':
   k.box(0,.02,0,2.,.05,4.,k.mortar)
   for z in [-.8,.8]:k.box(0,.04,z,2.,.015,.08,k.wood)
  else:
   if name=='roman_bridge':
    for z in [-1.8,1.8]:k.masonry(-.9,z,.25,.3,-2.,2.1)
    k.box(0,.08,0,2.,.18,4.,k.stone[3])
   for i in range(4):
    for j in range(8):k.box(-.75+i*.5,.2 if name=='roman_bridge' else .04,-1.75+j*.5,.47,.08,.47,k.stone[(i+2*j)%7])
   for z in [-1.9,1.9]:k.box(0,.5 if name=='roman_bridge' else .1,z,2.,.65 if name=='roman_bridge' else .14,.12,k.trim)
 elif name=='aqueduct':
  for x in [-6.,-2.,2.,6.]:k.masonry(x,0,.9,1.4,0.,4.3)
  for center in [-4.,0.,4.]:
   for i in range(18):
    a=math.pi*i/18;b=math.pi*(i+1)/18;beam((center+1.7*math.cos(a),3.3+1.7*math.sin(a),0),(center+1.7*math.cos(b),3.3+1.7*math.sin(b),0),.5,k.stone[i%7])
  k.masonry(0,0,14.,1.4,5.,1.)
  for z in [-.65,.65]:k.box(0,6.1,z,14.,.35,.16,k.trim)
  water=k.material('Aqueduct water',(.16,.43,.47));k.box(0,6.01,0,13.8,.025,1.,water)
 elif name in ['colosseum','theater']:
  full=name=='colosseum';count=40 if full else 24;span=math.tau if full else math.pi;radius=7.2 if full else 5.4
  for tier in range(3):
   y=tier*2.2
   for i in range(count):
    a=span*i/count;x=math.cos(a)*radius;z=math.sin(a)*radius
    k.box(x,y+1.1,z,.5,2.2,.6,k.stone[i%7],-a)
    aa=span*(i+.5)/count;k.box(math.cos(aa)*radius,y+2.1,math.sin(aa)*radius,.55,.25,1.3,k.trim,-aa)
   for ring in range(3):
    rr=radius-.8-ring*.45
    for i in range(count):
     a=span*i/count;k.box(math.cos(a)*rr,y+.2+ring*.12,math.sin(a)*rr,.5,.16,.9,k.stone[(i+ring)%7],-a)
  if not full:k.masonry(0,-.6,10.,.5,0.,3.5);shelter(0,-2.2,10.,3.)
 elif name=='statue':
  k.masonry(0,0,3.,3.,0.,1.5);k.box(0,1.6,0,3.3,.2,3.3,k.trim)
  k.cylinder(0,2.7,0,.55,2.1,k.trim,12,top_radius=.7);k.cylinder(0,4.1,0,.36,.62,k.trim,16)
  for x in [-.32,.32]:k.box(x,1.9,.15,.32,.65,.5,k.trim)
  beam((-.6,3.3,0),(-1.2,2.6,.2),.2,k.trim);beam((.6,3.3,0),(1.1,4.1,0),.2,k.trim);beam((1.1,2.,0),(1.1,4.7,0),.06,k.gold)
 elif name in ['weapon_smith','tool_smith']:
  shelter(0,-1.2,7.6,5.2);k.masonry(-2.,-2.,2.4,2.4,0.,1.3);k.cylinder(-2.,1.75,-2.,1.1,1.,k.roof[1],16,top_radius=.4);k.cylinder(-2.,3.,-2.,.25,2.,k.stone[2]);k.box(-2.,1.,-.82,.8,.5,.1,k.black)
  fire=k.material('Forge fire',(.95,.36,.06));k.box(-2.,1.,-.75,.55,.18,.12,fire)
  for x in [0.,2.3]:k.box(x,.8,1.6,.5,.9,.5,k.wood);k.box(x,1.35,1.6,1.,.25,.35,k.gold)
  k.box(1.,.9,-1.,3.,.15,1.1,k.wood)
  for x in [.2,.8,1.4,2.]:k.box(x,1.1,-1.,.12,.08,.65,k.gold)
 elif name in ['library','hospital','baths','temple']:
  if name=='temple':
   for i in range(4):k.box(0,.12*i,4.2-i*.3,8.-i*.2,.2,2.,k.trim)
   for x in [-3.8,-2.3,-.8,.8,2.3,3.8]:k.cylinder(x,2.4,2.,.22,4.,k.trim)
   shelter(0,-1.,9.2,7.5);k.masonry(0,-4.,8.8,.4,.4,3.8);k.box(0,1.1,0,1.4,1.5,1.4,k.trim)
  else:
   shelter(-3.,-1.,4.5,d-1.);shelter(3.,-1.,4.5,d-1.);k.masonry(0,-d/2+.3,w-.5,.35,0.,3.2)
   if name=='baths':
    water=k.material('Bath water',(.12,.46,.53));k.box(0,.22,1.,4.5,.18,5.,water)
    for x in [-2.4,2.4]:k.box(x,.35,1.,.2,.4,5.4,k.trim)
   elif name=='hospital':
    for x in [-3.3,3.3]:
     for z in [-3.,-1.,1.,3.]:k.box(x,.65,z,1.6,.18,.8,k.wood);k.box(x,.8,z,1.5,.14,.75,k.trim)
   else:
    for x in [-4.,4.]:
     for y in [.7,1.4,2.1]:
      k.box(x,y,-1.,1.4,.13,5.8,k.wood)
      for z in [-3.,-2.,-1.,0.,1.]:k.cylinder(x,y+.2,z,.13,.8,k.trim,8)
 elif name=='mercenary_camp':
  cloth=k.red
  for x in [-2.8,2.8]:
   k.box(x,1.,0,.12,2.,5.,k.wood)
   points=[(x,1.8,-2.8),(0,3.2,-2.8),(0,3.2,2.8),(x,1.8,2.8)];k.face(points,cloth);k.face(points[::-1],cloth);k.box(x,.9,0,.06,1.8,5.6,cloth)
  for z in [-2.8,2.8]:k.cylinder(0,1.6,z,.07,3.2,k.wood)
  k.box(0,.25,2.,2.8,.3,.6,k.wood)
 elif name in ['vineyard','olive_grove']:
  shelter(-3.6,3.4,3.5,2.6)
  for x in [-3.,0.,3.]:
   for z in [-3.,0.]:
    if name=='olive_grove':
     k.cylinder(x,1.,z,.16,2.,k.wood);k.cylinder(x,2.2,z,1.3,1.5,k.leaf,10,top_radius=.7)
     for dx in [-.6,0.,.6]:k.cylinder(x+dx,2.1,z+.8,.07,.12,k.black,8)
    else:
     for zz in [z-1.,z+1.]:k.box(x,1.1,zz,.1,2.,.1,k.wood)
     k.box(x,1.8,z,.07,.07,2.2,k.wood)
     for zz in [z-.7,z,z+.7]:k.cylinder(x,1.7,zz,.5,.4,k.leaf,8)
 elif name in ['olive_press','vintners_workshop']:
  shelter(0,-.5,7.6,6.2)
  for x in [-2.,2.]:
   k.cylinder(x,.5,0,1.1,.65,k.stone[2],18);k.box(x,1.8,0,2.8,.2,.2,k.wood)
   for dx in [-1.,1.]:k.box(x+dx,1.2,0,.16,2.,.16,k.wood)
   k.cylinder(x,1.1,0,.75,.25,k.stone[4],16)
  for x in [-2.,0.,2.]:k.cylinder(x,.6,2.3,.3,1.,k.roof[3],12,top_radius=.17)
 elif name=='wheat_farm':
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
 elif name=='arsenal':
  shelter(0,-.5,9.6,6.5);k.masonry(0,-3.6,9.4,.35,0.,3.2)
  for x in [-3.5,0.,3.5]:
   for z in [-2.5,.8]:k.box(x,1.1,z,2.5,.13,.45,k.wood)
   for a in [-.8,-.4,0.,.4,.8]:
    k.box(x+a,1.3,.8,.08,1.6,.08,k.gold);k.box(x+a,1.55,.8,.36,.1,.12,k.wood)
   for a in [-.7,.1,.9]:k.cylinder(x+a,.8,2.,.34,.14,k.roof[2],16)
 elif name=='research_center':
  shelter(0,-1.,9.5,7.6);k.masonry(0,-4.5,9.,.35,0.,3.2)
  for x in [-3.,0.,3.]:
   k.box(x,1.1,0,2.,.16,1.4,k.wood)
   for z in [-.4,.1,.5]:k.cylinder(x,1.25,z,.12,1.,k.trim,10)
   k.box(x,.4,2.5,1.,.7,.6,k.stone[2])
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
 lo=[-w*50.,0.,-d*50.];height={'dirt_road':8.,'stone_road':18.,'roman_bridge':90.,'aqueduct':650.,'colosseum':700.,'theater':700.}.get(name,500. if name=='siege_workshop' else 480.);hi=[w*50.,height,d*50.];(folder/'collision.glb').write_bytes(t.box_collision(lo,hi));return lo,hi
if __name__=='__main__':
 catalog=json.loads((ROOT/'building-models.json').read_text(encoding='utf8'));bounds={}
 for item in catalog:
  if item.get('procedural')=='roman_industry':bounds[item['id']]=generate(item)
 for name in ['ballista','onager','stone_ballista','scorpion','ram','covered_ram','siege_tower','assault_ladder','pluteus']:
  reset();artillery(name);folder=ROOT/'assets/models/units'/name;folder.mkdir(parents=True,exist_ok=True);save_engine(folder)
 generate_wheat_patch()
 # Keep the static catalog in agreement with these authored collision bounds.
 p=ROOT.parent/'shared/src/building_catalog.rs';s=p.read_text(encoding='utf8')
 for name,(lo,hi) in bounds.items():
  pat=r'(    Definition \{\s*name: "'+name+r'",.*?\},)';m=re.search(pat,s,re.S);assert m,name;block=re.sub(r'min: \[.*?\],','min: '+repr(lo)+',',m.group(),flags=re.S);block=re.sub(r'max: \[.*?\],','max: '+repr(hi)+',',block,flags=re.S);grid=next(x['grid'] for x in catalog if x['id']==name);block=re.sub(r'grid: \[.*?\]','grid: '+repr(grid),block,flags=re.S);s=s[:m.start()]+block+s[m.end():]
 p.write_text(s,encoding='utf8',newline='\n')
