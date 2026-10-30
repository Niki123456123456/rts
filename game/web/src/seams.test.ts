import assert from 'node:assert/strict';
import {world,status,until,delay} from './test-world';
const w=await world(),{db,game,own,build,lord}=w;
const call=(name:string,...args:any[])=>status(db,c=>(db.reducers as any)[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>(db.reducers as any)[name](...args));
try {
 assert.equal((await build('housing',23,-19)).tag,'Committed');
 assert.equal((await build('stone_wall_low',19,-19)).tag,'Committed','wall end joins neighbouring house across the raster remainder');
 const x=20.45;
 assert.equal((await call('setTargets',[lord().id],x,-25)).tag,'Committed');
 await until(()=>Math.hypot(lord().x-x,lord().z+25)<.3,'lord reaches south side of joined wall',45000);
 assert.equal((await call('setTargets',[lord().id],x,-13)).tag,'Committed');
 const end=Date.now()+18000;while(Math.hypot(lord().x-x,lord().z+13)>.3){assert.ok(Date.now()<end,'lord routes around solid extension');const u=lord();assert.ok(!(u.x>20.02&&u.x<20.85&&u.z>-20.05&&u.z<-17.95),'unit never crosses the former visible wall/house gap');await delay(30)}
 assert.equal((await build('tower_round_short',45,-15)).tag,'Committed');let towers=own().filter(b=>b.kind==='tower_round_short');const a=towers[0];
 assert.equal((await build('tower_round_short',a.x+10.843365173339845,a.z)).tag,'Committed');towers=own().filter(b=>b.kind==='tower_round_short').sort((a,b)=>a.x-b.x);const b=towers[1];
 const entry=await call('moveToRoof',[lord().id],a.id,a.x,a.z);assert.equal(entry.tag,'Committed',entry.tag==='Failed'?entry.value:entry.tag);
 await until(()=>Math.abs(lord().y-11.472)<.05,'lord reaches first round tower roof',50000);await delay(3500);
 assert.equal((await call('moveToRoof',[lord().id],b.id,b.x,b.z)).tag,'Committed');
 let crossed=false;const stop=Date.now()+20000;while(db.db.towerOccupant.unitId.find(lord().id)?.buildingId!==b.id){assert.ok(Date.now()<stop,'round tower roof transition completes');assert.ok(lord().y>11.4,'unit stays on connected roof');if(lord().x>a.x+3.3&&lord().x<b.x-3.3)crossed=true;await delay(30)}assert.ok(crossed,'unit used the filled floor between the round towers');
 assert.equal((await call('removeBuilding',b.id)).tag,'Committed');assert.ok(!own().some(t=>t.id===b.id));
 console.log('PASS: wall seals against ordinary building with no traversable slit; round towers have a continuous physical roof transition, no descent, and neighbouring tower removal succeeds.');
}finally{db.disconnect()}
