"""Create two combat variants from the packaged sword rig, plus an animated horse.
No external model source is needed; all geometry and clips are embedded in GLB.
"""
import json,math,struct,copy
from pathlib import Path
ROOT=Path(__file__).resolve().parent
SCALE=.007653524

def quat(axis,a):return [*(v*math.sin(a/2) for v in axis),math.cos(a/2)]
def mul(a,b):
 x,y,z,w=a;X,Y,Z,W=b
 return [w*X+x*W+y*Z-z*Y,w*Y-x*Z+y*W+z*X,w*Z+x*Y-y*X+z*W,w*W-x*X-y*Y-z*Z]
def inv(q):return [-q[0],-q[1],-q[2],q[3]]
def rotate(q,p):return mul(mul(q,[*p,0]),inv(q))[:3]
def source():
 raw=(ROOT/'assets/models/units/lord/mesh.glb').read_bytes();n=struct.unpack_from('<I',raw,12)[0]
 return json.loads(raw[20:20+n]),bytearray(raw[28+n:])
def generate(mounted):
 j,b=source()
 def accessor(values,kind,integer=False):
  while len(b)%4:b.append(0)
  offset=len(b);flat=[x for v in values for x in (v if isinstance(v,(list,tuple)) else [v])]
  b.extend(struct.pack('<'+('I' if integer else 'f')*len(flat),*flat));view=len(j['bufferViews']);j['bufferViews'].append({'buffer':0,'byteOffset':offset,'byteLength':len(b)-offset})
  a={'bufferView':view,'componentType':5125 if integer else 5126,'count':len(values),'type':kind}
  if kind=='VEC3':a.update(min=[min(v[k] for v in values) for k in range(3)],max=[max(v[k] for v in values) for k in range(3)])
  if kind=='SCALAR':a.update(min=[min(values)],max=[max(values)])
  j['accessors'].append(a);return len(j['accessors'])-1
 def material(name,color):
  j['materials'].append({'name':name,'pbrMetallicRoughness':{'baseColorFactor':[*color,1],'roughnessFactor':.85}});return len(j['materials'])-1
 def ellipse(name,center,radii,mat,parent=None):
  vertices=[];normals=[];indices=[];rows=10;cols=16
  for i in range(rows+1):
   phi=math.pi*i/rows
   for k in range(cols+1):
    a=2*math.pi*k/cols;n=(math.sin(phi)*math.cos(a),math.cos(phi),math.sin(phi)*math.sin(a));vertices.append([n[t]*radii[t] for t in range(3)]);nn=[n[t]/radii[t] for t in range(3)];length=math.sqrt(sum(v*v for v in nn));normals.append([v/length for v in nn])
  for i in range(rows):
   for k in range(cols):
    a=i*(cols+1)+k;c=a+cols+1;indices.extend([a,a+1,c,a+1,c+1,c])
  mesh=len(j['meshes']);j['meshes'].append({'name':name,'primitives':[{'attributes':{'POSITION':accessor(vertices,'VEC3'),'NORMAL':accessor(normals,'VEC3')},'indices':accessor(indices,'SCALAR',True),'material':mat}]})
  node=len(j['nodes']);j['nodes'].append({'name':name,'mesh':mesh,'translation':list(center)})
  if parent is None:j['scenes'][0]['nodes'].append(node)
  else:j['nodes'][parent].setdefault('children',[]).append(node)
  return node
 # Neutral ochre uniform differentiates the lighter units from the lord.
 for m in j['materials'][:3]:m.setdefault('pbrMetallicRoughness',{})['baseColorFactor']=[.88,.83,.68,1]
 if mounted:
  j['nodes'][33]['translation']=[0,70,0]
  brown=material('Horse chestnut coat',(.34,.14,.065));dark=material('Mane tail hooves',(.065,.04,.025));leather=material('Saddle',(.21,.07,.035));eye=material('Eyes',(.008,.008,.006))
  ellipse('Horse_body',(0,140,-5),(43,44,85),brown)
  ellipse('Horse_neck',(0,174,62),(27,61,30),brown)
  ellipse('Horse_head',(0,218,97),(23,28,43),brown)
  ellipse('Muzzle',(0,207,129),(20,20,22),brown)
  for x in [-15,15]:ellipse('Ear',(x,248,84),(6,19,9),brown)
  for x in [-22,22]:ellipse('Eye',(x,225,108),(3,4,5),eye)
  ellipse('Mane',(0,190,40),(9,53,12),dark)
  ellipse('Saddle',(0,177,-11),(43,9,35),leather)
  ellipse('Tail',(0,119,-97),(10,47,13),dark)
  legs=[]
  for x in [-28,28]:
   for z in [-56,50]:
    pivot=len(j['nodes']);j['nodes'].append({'name':'Horse_leg_joint','translation':[x,138,z],'children':[]});j['scenes'][0]['nodes'].append(pivot)
    ellipse('Horse_leg_upper',(0,-34,0),(12,40,13),brown,pivot);ellipse('Horse_leg_lower',(0,-93,0),(7,29,8),brown,pivot);ellipse('Hoof',(0,-128,4),(10,9,15),dark,pivot);legs.append((pivot,1 if x*z>0 else -1))
  parents={c:i for i,n in enumerate(j['nodes']) for c in n.get('children',[])};rest={}
  def visit(i,q):
   n=j['nodes'][i];q=mul(q,n.get('rotation',[0,0,0,1]));rest[i]=q
   for c in n.get('children',[]):visit(c,q)
  for i in j['scenes'][0]['nodes']:visit(i,[0,0,0,1])
  for animation in j['animations']:
   for channel in animation['channels']:
    node=channel['target']['node']
    if node in [17,18,25,26] and channel['target']['path']=='rotation':
     sampler=animation['samplers'][channel['sampler']];a=j['accessors'][sampler['output']];q=j['nodes'][node].get('rotation',[0,0,0,1]);angle=-1.03 if node in [18,26] else 1.3;q=mul(quat(rotate(inv(rest[parents[node]]),[1,0,0]),angle),q);sampler['output']=accessor([q]*a['count'],'VEC4')
   frames=33;duration={'idle':2.4,'walking':.72,'fighting':1.1}[animation['name']];inp=accessor([duration*i/(frames-1) for i in range(frames)],'SCALAR')
   for node,phase in legs:
    amplitude=.55 if animation['name']=='walking' else .09 if animation['name']=='fighting' else .015
    out=accessor([quat([1,0,0],amplitude*phase*math.sin(2*math.pi*i/(frames-1))) for i in range(frames)],'VEC4');animation['channels'].append({'sampler':len(animation['samplers']),'target':{'node':node,'path':'rotation'}});animation['samplers'].append({'input':inp,'output':out,'interpolation':'LINEAR'})
 if mounted:
  scene_root=len(j['nodes']);j['nodes'].append({'name':'LightCavalryRoot','children':j['scenes'][0]['nodes'][:]});j['scenes'][0]['nodes']=[scene_root]
 name='light_cavalry' if mounted else 'light_swordsman';folder=ROOT/'assets/models/units'/name;folder.mkdir(parents=True,exist_ok=True)
 j['buffers']=[{'byteLength':len(b)}];encoded=json.dumps(j,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4);b.extend(b'\0'*(-len(b)%4));(folder/'mesh.glb').write_bytes(struct.pack('<III',0x46546c67,2,28+len(encoded)+len(b))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(b),0x004e4942)+b)
 print(name,len(b),'bytes, three animations')
if __name__=='__main__':
 generate(False);generate(True)
