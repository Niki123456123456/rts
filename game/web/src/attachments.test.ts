import assert from 'node:assert/strict';
import {world,status,until,delay} from './test-world';
const w=await world(),{db,own,build,lord}=w;
const call=(name:string,...args:any[])=>status(db,c=>(db.reducers as any)[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>(db.reducers as any)[name](...args));
const stock=()=>db.db.playerResources.connectionId.find(db.connectionId)!;
try {
 assert.equal((await call('buildWallKind',19,-19,29,-19,'stone_wall_low')).tag,'Committed');assert.equal(stock().stone,24);
 assert.equal((await build('tower_stairwell',22,-18)).tag,'Committed','tower replaces two wall cells');assert.equal(stock().stone,14,'two replaced low walls refunded');
 assert.equal((await build('tower_stairwell',26,-18)).tag,'Committed','towers touch without a gap');assert.equal(stock().stone,4);
 const before=stock().stone;assert.equal((await call('buildWallKind',17,-19,31,-19,'stone_wall_low')).tag,'Committed');assert.equal(before-stock().stone,2,'drag charges only uncovered new wall cells');
 assert.equal(own().filter(b=>b.kind==='stone_wall_low').length,4);
 const wood=stock().wood;assert.equal((await call('buildWallKind',19,-29,23,-29,'wooden_wall')).tag,'Committed');assert.equal(stock().wood,wood-6);
 const towers=own().filter(b=>b.kind==='tower_stairwell').sort((a,b)=>a.x-b.x);
 assert.equal((await call('moveToRoof',[lord().id],towers[0].id,22,-18)).tag,'Committed');
 await until(()=>db.db.towerOccupant.unitId.find(lord().id)?.buildingId===towers[0].id&&lord().y>5,'lord reaches first tower',50000);await delay(7000);
 assert.equal((await call('moveToRoof',[lord().id],towers[1].id,26,-18)).tag,'Committed');
 const end=Date.now()+15000;while(db.db.towerOccupant.unitId.find(lord().id)?.buildingId!==towers[1].id){assert.ok(Date.now()<end,'joined roof transition completes');assert.ok(lord().y>5,'unit crosses on roof without descending');await delay(40)}
 console.log('PASS: walls attach directly, anchors refund replaced walls, dragging through towers charges only new sections, wood-only palisade, seamless adjacent-tower roof crossing.');
} finally {db.disconnect()}
const a=await world();const d=a.db;const act=(name:string,...args:any[])=>status(d,c=>(d.reducers as any)[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>(d.reducers as any)[name](...args));
try {
 assert.equal((await a.build('gatehouse_small',25,25)).tag,'Committed');
 assert.equal((await a.build('apple_farm',0,24)).tag,'Committed');
 const gate=a.own().find(b=>b.kind==='gatehouse_small')!,keep=a.own().find(b=>b.kind==='keep')!;
 await until(()=>Array.from(d.db.worker.iter()).some(w=>w.gameId===a.game),'farmer assigned',18000);
 const worker=Array.from(d.db.worker.iter()).find(w=>w.gameId===a.game)!;
 await delay(10000);assert.equal((await act('setGate',gate.id,true)).tag,'Committed');
 assert.equal((await act('closeAllGates')).tag,'Committed');assert.equal(d.db.gateState.buildingId.find(gate.id)!.open,false);
 assert.equal((await act('setGate',gate.id,true)).tag,'Committed');
 assert.equal((await act('palaceAlarm',true)).tag,'Committed');
 await until(()=>d.db.towerOccupant.unitId.find(worker.unitId)?.buildingId===keep.id,'farmer physically seeks palace',45000);
 await until(()=>d.db.gateState.buildingId.find(gate.id)?.open===false,'alarm closes gates after residents enter',30000);
 assert.ok(d.db.armyUnit.id.find(worker.unitId)!.y>5);
 assert.equal((await act('palaceAlarm',false)).tag,'Committed');
 await until(()=>!d.db.towerOccupant.unitId.find(worker.unitId),'farmer descends after all clear',30000);
 console.log('PASS: manual one-click gate closure, alarm pauses work, farmer climbs palace stairs, gates close after sheltering, all-clear returns farmer to ground.');
} finally {d.disconnect()}
