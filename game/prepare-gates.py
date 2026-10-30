"""Remove the fixed gate leaves. GateState supplies the animated, collidable replacement."""
import json,struct
from pathlib import Path
ROOT=Path(__file__).resolve().parent
def prepare(data,kind):
    length=struct.unpack_from('<I',data,12)[0];doc=json.loads(data[20:20+length]);tail=data[20+length:]
    if doc.get('extras',{}).get('rts_operable_gate'):return data
    primitive={'gatehouse_small':7,'gatehouse_medium':6,'gatehouse_large':8}.get(kind)
    if primitive is None:return data
    del doc['meshes'][0]['primitives'][primitive]
    doc.setdefault('extras',{})['rts_operable_gate']=True
    encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4)
    return struct.pack('<III',0x46546c67,2,20+len(encoded)+len(tail))+struct.pack('<II',len(encoded),0x4e4f534a)+encoded+tail
if __name__=='__main__':
    for kind in ['gatehouse_small','gatehouse_medium','gatehouse_large']:
        p=ROOT/'assets/models/buildings/fortifications'/kind/'mesh.glb';p.write_bytes(prepare(p.read_bytes(),kind))
