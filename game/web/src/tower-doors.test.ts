import assert from 'node:assert/strict';
import {world,status,until,delay} from './test-world';
const {db,own,build,lord}=await world();
const call=(name:string,...args:any[])=>status(db,c=>(db.reducers as any)[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>(db.reducers as any)[name](...args));
const ok=(s:any,label:string)=>assert.equal(s.tag,'Committed',`${label}: ${s.value??s.tag}`);
try {
 ok(await build('stone_wall_crenellated',19,-25),'default crenels');let wall=own().find(b=>b.kind==='stone_wall_crenellated')!;assert.ok(wall);
 ok(await build('stone_wall_low',19,-23),'low inner layer');assert.equal(own().find(b=>b.id===wall.id)!.kind,'stone_wall_crenellated_low');const low=own().find(b=>b.kind==='stone_wall_low')!;
 ok(await call('removeBuilding',low.id),'remove inner layer');assert.equal(own().find(b=>b.id===wall.id)!.kind,'stone_wall_crenellated');
 ok(await call('removeBuilding',wall.id),'remove isolated crenels');
 ok(await build('tower_stairwell',22,-18),'higher tower');const tower=own().find(b=>b.kind==='tower_stairwell')!;
 ok(await build('stone_wall_low',25,-17),'directly adjacent low wall');wall=own().find(b=>b.kind==='stone_wall_low')!;
 ok(await call('moveToRoof',[lord().id],tower.id,tower.x,tower.z),'enter higher tower');
 await until(()=>db.db.towerOccupant.unitId.find(lord().id)?.buildingId===tower.id&&Math.abs(lord().y-7.413)<.06,'lord reaches high roof',50000);await delay(3000);
 ok(await call('moveToRoof',[lord().id],wall.id,wall.x,wall.z),'higher tower through door onto lower wall');
 await until(()=>db.db.towerOccupant.unitId.find(lord().id)?.buildingId===wall.id&&Math.abs(lord().y-4.05)<.06,'lord reaches low wall without ground entry',20000);await delay(1500);
 ok(await call('moveToRoof',[lord().id],tower.id,tower.x,tower.z),'lower wall through door into higher tower');
 await until(()=>db.db.towerOccupant.unitId.find(lord().id)?.buildingId===tower.id&&Math.abs(lord().y-7.413)<.06,'lord returns to higher roof through door',20000);
 console.log('PASS: crenels default to high wall height, adjacent low skin changes height and removal restores it; troops transfer down and up through higher tower doorway.');
}finally{db.disconnect()}
