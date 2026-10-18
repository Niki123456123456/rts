#!/usr/bin/env python3
"""Package original soldier rigs, hand-held weapons and authored in-place skeletal clips."""
import json,math,struct
from pathlib import Path
ROOT=Path(__file__).resolve().parent
SOURCE=ROOT.parent/'assets/models/meshes/units/models'
def read(name):
 b=(SOURCE/(name+'.glb')).read_bytes();n=struct.unpack_from('<I',b,12)[0]
 return json.loads(b[20:20+n]),bytearray(b[28+n:])
def mul(a,b):
 x,y,z,w=a;X,Y,Z,W=b
 return [w*X+x*W+y*Z-z*Y,w*Y-x*Z+y*W+z*X,w*Z+x*Y-y*X+z*W,w*W-x*X-y*Y-z*Z]
def inv(q):return [-q[0],-q[1],-q[2],q[3]]
def rotate(q,p):return mul(mul(q,[*p,0]),inv(q))[:3]
def axis(v,a):return [*(x*math.sin(a/2) for x in v),math.cos(a/2)]
def globals_(j,poses=None):
 out={}
 def visit(i,q):
  n=j['nodes'][i];q=mul(q,(poses or {}).get(i,n.get('rotation',[0,0,0,1])));out[i]=q
  for c in n.get('children',[]):visit(c,q)
 for i in j['scenes'][0]['nodes']:visit(i,[0,0,0,1])
 return out
for name,weapon,height in [('lord','lord_sword',1.85),('armed_peasant','armed_peasant_fork',1.75)]:
 j,b=read(name);w,wb=read(weapon)
 # Preserve skin and original geometry; bake no changes into the source files.
 parents={c:i for i,n in enumerate(j['nodes']) for c in n.get('children',[])}
 rest=globals_(j); names={n['name']:i for i,n in enumerate(j['nodes']) if 'mesh' not in n and (n['name'] != 'Bip01' or 'translation' in n)}
 hand=names['Bip01 R Hand'];grip_reference=None
 weapon_pitch=math.pi-.5 if name=='lord' else .45
 weapon_q=axis([1,0,0],weapon_pitch)
 def pose(state,t):
  wave=math.sin(2*math.pi*t);pose={}
  def turn(n,rotations):
   i=names[n];q=j['nodes'][i].get('rotation',[0,0,0,1]);pq=rest[parents[i]]
   for v,a in rotations:q=mul(axis(rotate(inv(pq),v),a),q)
   pose[i]=q
  walk=state=='walking';fight=state=='fighting';s=wave if walk else 0
  turn('Bip01 R UpperArm',[([0,0,1],1.08),([1,0,0],-.22*s+(-.65-.3*wave if fight else .05))])
  turn('Bip01 L UpperArm',[([0,0,1],-1.08),([1,0,0],.22*s+(-.55 if fight else .05))])
  turn('Bip01 R Forearm',[([1,0,0],-.18-(.35*(1-wave) if fight else 0))])
  turn('Bip01 L Forearm',[([1,0,0],-.2-(.3 if fight else 0))])
  turn('Bip01 L Thigh',[([1,0,0],.48*s)])
  turn('Bip01 R Thigh',[([1,0,0],-.48*s)])
  turn('Bip01 L Calf',[([1,0,0],-.6*max(0,-s))])
  turn('Bip01 R Calf',[([1,0,0],-.6*max(0,s))])
  turn('Bip01 Spine',[([1,0,0],.015*wave+(.12 if fight else 0)),([0,1,0],.18*wave if fight else .025*s)])
  if grip_reference is not None:
   # +Z is the model's front. Keep the blade/tines forward while the arms
   # swing and thrust; compensate at the wrist, leaving the weapon rigidly
   # attached to the animated hand rather than turning it independently.
   pitch=(.95+.4*wave if name=='lord' else 1.05+.3*wave) if fight else weapon_pitch+(.1 if walk else .025)*wave
   desired=mul(axis([0,1,0],.16*wave if fight else 0),axis([1,0,0],pitch))
   hand_world=mul(mul(desired,inv(weapon_q)),grip_reference)
   parent_world=globals_(j,pose)[parents[hand]]
   pose[hand]=mul(inv(parent_world),hand_world)
  return pose
 grip_reference=globals_(j,pose('idle',0))[hand]
 # Merge the weapon's embedded images, materials and mesh buffers into the rig.
 while len(b)%4:b.append(0)
 offset=len(b);b.extend(wb)
 offsets={k:len(j.get(k,[])) for k in ['bufferViews','accessors','images','samplers','textures','materials','meshes']}
 for v in w.get('bufferViews',[]):v['buffer']=0;v['byteOffset']=v.get('byteOffset',0)+offset
 for a in w.get('accessors',[]):
  if 'bufferView' in a:a['bufferView']+=offsets['bufferViews']
 for a in w.get('images',[]):
  if 'bufferView' in a:a['bufferView']+=offsets['bufferViews']
 for a in w.get('textures',[]):
  if 'source' in a:a['source']+=offsets['images']
  if 'sampler' in a:a['sampler']+=offsets['samplers']
 def remap_texture(obj):
  if isinstance(obj,dict):
   for k,v in obj.items():
    if k.endswith('Texture') and isinstance(v,dict) and 'index' in v:v['index']+=offsets['textures']
    else:remap_texture(v)
  elif isinstance(obj,list):
   for v in obj:remap_texture(v)
 for m in w.get('materials',[]):remap_texture(m)
 for m in w.get('meshes',[]):
  for p in m['primitives']:
   p['attributes']={k:v+offsets['accessors'] for k,v in p['attributes'].items()}
   if 'indices' in p:p['indices']+=offsets['accessors']
   if 'material' in p:p['material']+=offsets['materials']
 for k in offsets:j.setdefault(k,[]).extend(w.get(k,[]))
 hq=grip_reference
 j['nodes'][hand].setdefault('children',[]).append(len(j['nodes']))
 j['nodes'].append({'name':'Weapon_'+weapon,'mesh':offsets['meshes'],'translation':[8,0,0], 'rotation':mul(inv(hq),weapon_q),'scale':[.65]*3})
 for m in j.get('materials',[]):m['alphaMode']='OPAQUE'
 def accessor(values,kind):
  while len(b)%4:b.append(0)
  start=len(b);flat=[x for v in values for x in (v if isinstance(v,list) else [v])]
  b.extend(struct.pack('<'+'f'*len(flat),*flat));view=len(j['bufferViews']);j['bufferViews'].append({'buffer':0,'byteOffset':start,'byteLength':len(b)-start})
  a={'bufferView':view,'componentType':5126,'count':len(values),'type':kind}
  if kind=='SCALAR':a.update(min=[min(values)],max=[max(values)])
  index=len(j['accessors']);j['accessors'].append(a);return index
 j['animations']=[]
 for state,duration in [('idle',2.4),('walking',.72),('fighting',1.1)]:
  frames=33;times=[duration*i/(frames-1) for i in range(frames)];inp=accessor(times,'SCALAR');poses=[pose(state,i/(frames-1)) for i in range(frames)]
  animation={'name':state,'samplers':[],'channels':[]}
  for node in poses[0]:
   out=accessor([p[node] for p in poses],'VEC4');index=len(animation['samplers']);animation['samplers'].append({'input':inp,'output':out,'interpolation':'LINEAR'});animation['channels'].append({'sampler':index,'target':{'node':node,'path':'rotation'}})
  root=names['Bip01'];base=j['nodes'][root]['translation'];values=[[base[0],base[1]+(1.4*math.sin(4*math.pi*i/(frames-1)) if state=='walking' else .5*math.sin(2*math.pi*i/(frames-1))),base[2]] for i in range(frames)]
  out=accessor(values,'VEC3');animation['channels'].append({'sampler':len(animation['samplers']),'target':{'node':root,'path':'translation'}});animation['samplers'].append({'input':inp,'output':out,'interpolation':'LINEAR'});j['animations'].append(animation)
 j['buffers']=[{'byteLength':len(b)}]
 encoded=json.dumps(j,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4);b.extend(b'\0'*(-len(b)%4))
 out=ROOT/'assets/models/units'/name;out.mkdir(parents=True,exist_ok=True)
 (out/'mesh.glb').write_bytes(struct.pack('<III',0x46546c67,2,28+len(encoded)+len(b))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(b),0x004e4942)+b)
 original=j['meshes'][:offsets['meshes']];positions=[j['accessors'][p['attributes']['POSITION']] for m in original for p in m['primitives']];lo=min(a['min'][1] for a in positions);hi=max(a['max'][1] for a in positions)
 print(name,'scale',height/(hi-lo),'base',lo,'clips',len(j['animations']))
