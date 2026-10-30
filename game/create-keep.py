#!/usr/bin/env python3
"""Author the playable keep as a compact, material-batched glTF in centimetres.

No external image, Blender installation or original asset archive is required.
Individual masonry courses, roof tiles, columns and arch stones are geometry.
"""
import json
import math
from pathlib import Path
import random
import struct

ROOT = Path(__file__).resolve().parent
rng = random.Random(841)
materials = []
geometry = []

def material(name, color, metal=0):
    materials.append({'name': name, 'pbrMetallicRoughness': {
        'baseColorFactor': [*color, 1], 'metallicFactor': metal, 'roughnessFactor': .86}})
    geometry.append(([], [], []))
    return len(materials)-1

stone = [material('Honey limestone '+str(i), (.63+i*.023, .55+i*.022, .41+i*.021)) for i in range(7)]
trim = material('Carved pale limestone', (.84, .76, .60))
mortar = material('Recessed sandstone mortar', (.37, .32, .24))
red = material('Burgundy cloth', (.44, .035, .025))
gold = material('Banner embroidery and bronze', (.81, .53, .12), .4)
wood = material('Dark oak', (.23, .115, .047))
black = material('Door recess and charcoal', (.065, .046, .025))
roof = [material('Terracotta tile '+str(i), (.38+i*.043, .11+i*.018, .045+i*.012)) for i in range(5)]
leaf = material('Cypress needles', (.12, .23, .075))

def face(points, mat):
    vertices, normals, indices = geometry[mat]
    a,b,c = points[:3]
    u = [b[i]-a[i] for i in range(3)]; v = [c[i]-a[i] for i in range(3)]
    n = [u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
    length = math.sqrt(sum(x*x for x in n))
    if not length: return
    start=len(vertices)
    vertices.extend(tuple(x*100 for x in p) for p in points)
    normals.extend([tuple(x/length for x in n)]*len(points))
    for i in range(1,len(points)-1): indices.extend([start,start+i,start+i+1])

def box(x,y,z,w,h,d,mat, angle=0):
    # All dimensions are metres; optional rotation around vertical axis.
    points=[]
    for a,b,c in [(-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]:
        px,pz=a*w*.5,c*d*.5
        points.append((x+px*math.cos(angle)+pz*math.sin(angle),y+b*h*.5,z-px*math.sin(angle)+pz*math.cos(angle)))
    for ids in [(0,3,2,1),(4,5,6,7),(0,4,7,3),(1,2,6,5),(3,7,6,2),(0,1,5,4)]:
        face([points[i] for i in ids],mat)

def cylinder(x,y,z,r,h,mat,n=12,top_radius=None):
    rt=r if top_radius is None else top_radius
    bottom=[(x+r*math.cos(i*2*math.pi/n),y-h/2,z+r*math.sin(i*2*math.pi/n)) for i in range(n)]
    top=[(x+rt*math.cos(i*2*math.pi/n),y+h/2,z+rt*math.sin(i*2*math.pi/n)) for i in range(n)]
    for i in range(n):
        j=(i+1)%n; face([bottom[i],top[i],top[j],bottom[j]],mat)
    face(top[::-1],mat); face(bottom,mat)

def masonry(x,z,w,d,base,height):
    box(x,base+height/2,z,w-.06,height,d-.06,mortar)
    rows=round(height/.42)
    for row in range(rows):
        yy=base+(row+.5)*height/rows
        for zz in [-1,1]:
            count=math.ceil(w/.85)
            for col in range(count):
                xx=x-w/2+(col+.5)*w/count
                # Thin relief boxes reveal every stone and its mortar joint.
                box(xx,yy,z+zz*d/2,w/count-.026,height/rows-.026,.13,rng.choice(stone))
        for xx in [-1,1]:
            count=math.ceil(d/.85)
            for col in range(count):
                zz=z-d/2+(col+.5)*d/count
                box(x+xx*w/2,yy,zz,.13,height/rows-.026,d/count-.026,rng.choice(stone))
        # Offset alternate courses with slender scored joints on front/back.
        if row%2:
            for col in range(1,round(w/.85)):
                for zz in [-1,1]:
                    box(x-w/2+col*.85-.425,yy,z+zz*(d/2+.07),.018,height/rows-.026,.012,mortar)
    for yy in [base+.18,base+height-.12]: box(x,yy,z,w+.22,.23,d+.22,trim)

def arch(x,z,bottom,radius):
    # Voussoirs following a real semicircular arch, extruded forward.
    for i in range(11):
        a=i*math.pi/11; b=(i+1)*math.pi/11-.015
        for front in [False,True]:
            zz=z+(.16 if front else -.1)
            pts=[(x+r*math.cos(t),bottom+r*math.sin(t),zz) for r,t in [(radius,a),(radius+.32,a),(radius+.32,b),(radius,b)]]
            if not front: pts.reverse()
            face(pts,trim)
        # The narrow side faces make separate arch stones visible obliquely.
        for t in [a,b]: face([(x+radius*math.cos(t),bottom+radius*math.sin(t),z-.1),(x+(radius+.32)*math.cos(t),bottom+(radius+.32)*math.sin(t),z-.1),(x+(radius+.32)*math.cos(t),bottom+(radius+.32)*math.sin(t),z+.16),(x+radius*math.cos(t),bottom+radius*math.sin(t),z+.16)],rng.choice(stone))

def banner(x,z,y):
    box(x,y,z,1.3,3.7,.055,red)
    for dx in [-.60,.60]: box(x+dx,y,z+.04,.055,3.68,.025,gold)
    box(x,y+1.85,z,.13,.12,.28,gold)
    # Stylised gold laurel/sun emblem, not a copied crest.
    for i in range(12):
        a=i*math.tau/12
        box(x+.22*math.cos(a),y+.32*math.sin(a),z+.05,.075,.12,.028,gold,-a)
    box(x,y,z+.06,.08,.42,.03,gold)
    face([(x-.65,y-1.85,z),(x+.65,y-1.85,z),(x,y-2.12,z)],red)

def tiled_roof(x,z,w,d,y,rise):
    for sign in [-1,1]:
        for row in range(8):
            t0=row/8; t1=(row+1)/8
            for col in range(round(w/.34)):
                xx=x-w/2+col*.34
                zz0=z+sign*d/2*t0; zz1=z+sign*d/2*t1
                yy0=y+rise*(1-t0); yy1=y+rise*(1-t1)
                pts=[(xx,yy0+.025,zz0),(xx+.33,yy0+.025,zz0),(xx+.33,yy1+.025,zz1),(xx,yy1+.025,zz1)]
                if sign>0: pts.reverse()
                face(pts,rng.choice(roof))
                box(xx+.16,(yy0+yy1)/2,(zz0+zz1)/2,.07,.07,abs(zz1-zz0),rng.choice(roof))
    for i in range(round(w/.34)): box(x-w/2+i*.34,y+rise+.07,z,.33,.16,.27,roof[3])

def generate():
    # Central terrace supports the shared 5.4 m walkable roof.
    masonry(0,0,12,10,0,5.4)
    for x in range(-5,6):
        for z in range(-4,5): box(x,5.42,z,.97,.10,.97,rng.choice(stone))
    masonry(0,-5.5,10,3.0,5.4,5.0)
    tiled_roof(0,-5.5,10.7,4.2,10.5,2.15)
    # Flanking corner turrets frame the hall without covering the terrace.
    for x in [-6.2,6.2]:
        masonry(x,-4.8,2.5,3.0,0,9.1)
        for dx in [-.9,0,.9]:
            for zz in [-6.15,-3.45]: box(x+dx,9.5,zz,.58,.8,.5,trim)
        for zz in [-5.3,-4.35]:
            for dx in [-1.1,1.1]: box(x+dx,9.5,zz,.5,.8,.58,trim)
        box(x,7.45,-3.25,.42,1.25,.05,black)
    # Ground-floor portals and columns.
    for x in [-3.8,0,3.8]:
        box(x,1.65,5.08,1.8,3.3,.12,black)
        box(x,1.55,5.18,1.45,2.9,.08,wood)
        for dx in [-.52,0,.52]: box(x+dx,1.55,5.25,.035,2.85,.03,gold)
        arch(x,5.22,2.8,.96)
    for x in [-5.6,-2,2,5.6]:
        cylinder(x,2.35,5.35,.24,3.9,trim,16)
        for yy in [.4,4.3]: box(x,yy,5.35,.68,.24,.72,trim)
    for x in [-4,0,4]:
        box(x,7.3,-3.93,1.1,2.4,.08,black)
        arch(x,-3.77,7.95,.57)
    for x in [-2.0,2.0]: banner(x,-3.79,8.15)
    # Terrace balustrade, open on the east for the staircase.
    for x in [-5.9,5.9]:
        box(x,6.25,-.9,.35,.18,6.8,trim)
        for z in range(-3,3): cylinder(x,5.87,z,.095,.7,trim,8)
    box(0,6.25,4.75,11.7,.18,.38,trim)
    for i in range(20): cylinder(-5.6+i*.58,5.87,4.75,.095,.7,trim,8)
    # Real outside staircase. Shared navigation uses its centreline.
    for sign in [-1,1]:
        for step in range(27):
            h=5.4*(27-step)/27; z=3.0+(step+.5)*7.4/27
            box(sign*6.5,h/2,z,2.8,h,7.4/27-.008,rng.choice(stone))
        box(sign*5.25,5.30,3.0,2.6,.22,1.0,trim)
    # Low stone cheeks follow the stair incline.
    for side in [-8.0,-5.0,5.0,8.0]:
        for step in range(27):
            h=5.4*(27-step)/27
            box(side,h/2+.34,3.0+(step+.5)*7.4/27,.26,h+.68,7.4/27,trim)
    # Campfire and rustic benches on the forecourt, part of the same build plot.
    for i in range(16):
        a=i*math.tau/16
        box(1.15*math.cos(a),.22,11+1.15*math.sin(a),.44,.4,.36,rng.choice(stone),-a)
    cylinder(0,.06,11,.9,.1,black,16)
    for a in [.25,2.2,4.3]: box(.12*math.sin(a),.28,11,.2,.23,1.6,wood,a)
    for x,z,angle in [(-2.9,10.5,0),(2.9,12.0,0),(0,13.2,math.pi/2)]:
        box(x,.58,z,.50,.17,1.7,wood,angle)
        for sign in [-1,1]:
            dx=math.sin(angle)*sign*.6; dz=math.cos(angle)*sign*.6
            box(x+dx,.28,z+dz,.28,.55,.25,wood)
    # Terracotta pots and cypresses at the entrance.
    for x in [-3.,3.]:
        cylinder(x,.38,6.3,.46,.7,roof[2],12,top_radius=.57)
        cylinder(x,1.70,6.3,.48,2.0,leaf,10,top_radius=.06)
    write_model()
    minimum, maximum = [-800.,0.,-800.],[800.,1300.,800.]
    # A single collision box, separate from the detailed render mesh.
    import importlib.util
    spec=importlib.util.spec_from_file_location('building_tools',ROOT/'import-buildings.py')
    module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    folder=ROOT/'assets/models/buildings/fortifications/keep'
    (folder/'collision.glb').write_bytes(module.box_collision(minimum,maximum))
    return minimum, maximum

def write_model():
    binary=bytearray(); views=[]; accessors=[]; primitives=[]
    def accessor(data, shape, component=5126):
        if component==5126: raw=struct.pack('<'+'f'*sum(map(len,data)),*(v for p in data for v in p))
        else: raw=struct.pack('<'+'I'*len(data),*data)
        binary.extend(b'\0'*(-len(binary)%4)); offset=len(binary); binary.extend(raw)
        views.append({'buffer':0,'byteOffset':offset,'byteLength':len(raw),'target':34963 if component==5125 else 34962})
        record={'bufferView':len(views)-1,'componentType':component,'count':len(data),'type':shape}
        if shape=='VEC3':
            record.update(min=[min(p[i] for p in data) for i in range(3)],max=[max(p[i] for p in data) for i in range(3)])
        accessors.append(record); return len(accessors)-1
    for mat,(vertices,normals,indices) in enumerate(geometry):
        if not indices: continue
        primitives.append({'attributes':{'POSITION':accessor(vertices,'VEC3'),'NORMAL':accessor(normals,'VEC3')},'indices':accessor(indices,'SCALAR',5125),'material':mat})
    doc={'asset':{'version':'2.0','generator':'RTS masonry keep'},'scene':0,'scenes':[{'nodes':[0]}],
         'nodes':[{'mesh':0,'name':'Bergfried – Steine, Säulen, Ziegel und Feuerstelle'}],
         'meshes':[{'primitives':primitives}],'buffers':[{'byteLength':len(binary)}],
         'bufferViews':views,'accessors':accessors,'materials':materials}
    encoded=json.dumps(doc,separators=(',',':')).encode(); encoded+=b' '*(-len(encoded)%4)
    binary+=b'\0'*(-len(binary)%4)
    data=struct.pack('<III',0x46546c67,2,28+len(encoded)+len(binary))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+struct.pack('<II',len(binary),0x004e4942)+binary
    folder=ROOT/'assets/models/buildings/fortifications/keep'; folder.mkdir(parents=True,exist_ok=True)
    (folder/'mesh.glb').write_bytes(data)
    print(f'Keep: {sum(len(g[2])//3 for g in geometry):,} triangles, {len(primitives)} batches, {len(data):,} bytes')

if __name__=='__main__': generate()
