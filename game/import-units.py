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
def sub(a,b):return [x-y for x,y in zip(a,b)]
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def norm(a):return math.sqrt(dot(a,a))
def unit(a):return [x/max(norm(a),1e-9) for x in a]
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def from_to(a,b):
 a=unit(a);b=unit(b);d=dot(a,b)
 if d < -.9999:return axis(unit(cross(a,[0,1,0] if abs(a[1])<.9 else [1,0,0])),math.pi)
 q=[*cross(a,b),1+d];n=norm(q);return [x/n for x in q]
def bone_positions(j,poses=None):
 out={}
 def visit(i,q,p):
  n=j['nodes'][i];t=rotate(q,n.get('translation',[0,0,0]));p=[a+b for a,b in zip(p,t)]
  q=mul(q,(poses or {}).get(i,n.get('rotation',[0,0,0,1])));out[i]=(q,p)
  for c in n.get('children',[]):visit(c,q,p)
 for i in j['scenes'][0]['nodes']:visit(i,[0,0,0,1],[0,0,0])
 return out
for name,weapon,height in [('lord','lord_sword',1.85),('armed_peasant','armed_peasant_fork',1.75),('archer','archer_bow',1.8)]:
 j,b=read(name);w,wb=read(weapon)
 # Preserve skin and original geometry; bake no changes into the source files.
 parents={c:i for i,n in enumerate(j['nodes']) for c in n.get('children',[])}
 rest=globals_(j); names={n['name']:i for i,n in enumerate(j['nodes']) if 'mesh' not in n and (n['name'] != 'Bip01' or 'translation' in n)}
 hand=names['Bip01 L Hand' if name=='archer' else 'Bip01 R Hand'];grip_reference=None
 weapon_pitch=math.pi-.5 if name=='lord' else .45
 weapon_q=mul(axis([0,1,0],-math.pi/2),axis([1,0,0],-math.pi/2)) if name=='archer' else axis([1,0,0],weapon_pitch)
 rest_positions=bone_positions(j)
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
  if name=='archer' and fight:
   # Two-bone IK keeps the left hand on the bow and draws the right hand
   # from the string towards the cheek. All rotations remain on the rig.
   draw=.5-.5*math.cos(2*math.pi*t)
   for side,target,hint in [('L',[16,160,50],[1,0,0]),('R',[-8,160+10*draw,46-35*draw],[-1,0,0])]:
    upper=names['Bip01 '+side+' UpperArm'];fore=names['Bip01 '+side+' Forearm'];wrist=names['Bip01 '+side+' Hand']
    shoulder=bone_positions(j,pose)[upper][1]
    a=rest_positions[upper][1];e=rest_positions[fore][1];h=rest_positions[wrist][1]
    l1=norm(sub(e,a));l2=norm(sub(h,e));direction=unit(sub(target,shoulder));d=min(norm(sub(target,shoulder)),l1+l2-.01)
    target=[x+y*d for x,y in zip(shoulder,direction)]
    along=(l1*l1-l2*l2+d*d)/(2*d);bend=unit(sub(hint,[x*dot(hint,direction) for x in direction]));height=math.sqrt(max(0,l1*l1-along*along))
    elbow=[x+y*along+z*height for x,y,z in zip(shoulder,direction,bend)]
    upper_world=mul(from_to(sub(e,a),sub(elbow,shoulder)),rest_positions[upper][0])
    pose[upper]=mul(inv(bone_positions(j,pose)[parents[upper]][0]),upper_world)
    fore_world=mul(from_to(sub(h,e),sub(target,elbow)),rest_positions[fore][0])
    pose[fore]=mul(inv(upper_world),fore_world)
  if grip_reference is not None:
   # +Z is the model's front. Keep the blade/tines forward while the arms
   # swing and thrust; compensate at the wrist, leaving the weapon rigidly
   # attached to the animated hand rather than turning it independently.
   pitch=(.95+.4*wave if name=='lord' else 1.05+.3*wave) if fight else weapon_pitch+(.1 if walk else .025)*wave
   desired=mul(axis([1,0,0],.04*wave),weapon_q) if name=='archer' else mul(axis([0,1,0],.16*wave if fight else 0),axis([1,0,0],pitch))
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
 j['nodes'].append({'name':'Weapon_'+weapon,'mesh':offsets['meshes'],'translation':[0,0,0] if name=='archer' else [8,0,0], 'rotation':mul(inv(hq),weapon_q),'scale':[.65]*3})
 for m in j.get('materials',[]):m['alphaMode']='OPAQUE'
 def accessor(values,kind):
  while len(b)%4:b.append(0)
  start=len(b);flat=[x for v in values for x in (v if isinstance(v,list) else [v])]
  b.extend(struct.pack('<'+'f'*len(flat),*flat));view=len(j['bufferViews']);j['bufferViews'].append({'buffer':0,'byteOffset':start,'byteLength':len(b)-start})
  a={'bufferView':view,'componentType':5126,'count':len(values),'type':kind}
  if kind=='SCALAR':a.update(min=[min(values)],max=[max(values)])
  index=len(j['accessors']);j['accessors'].append(a);return index
 j['animations']=[]
 for state,duration in [('idle',2.4),('walking',.72),('fighting',1.25 if name=='archer' else 1.1)]:
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

# Projectiles retain the original arrow mesh/materials as a separate reusable scene.
import shutil
arrow_out=ROOT/'assets/models/units/archer/arrow.glb'
shutil.copyfile(SOURCE/'archer_arrow.glb',arrow_out)
