import assert from 'node:assert/strict';
import {world,connect,status,until,delay} from './test-world';
const {db,game,own,build,lord}=await world();const enemy=await connect();
const tower=!!process.env.RTS_SIEGE_TOWER;const accessKind=tower?'siege_tower':'assault_ladder';const wallKind=tower?'stone_wall_high':'stone_wall_low';
const call=(d:any,n:string,...a:any[])=>status(d,c=>d.reducers[`on${n[0].toUpperCase()+n.slice(1)}`](c),()=>d.reducers[n](...a));
const ok=(s:any,l:string)=>assert.equal(s.tag,'Committed',`${l}: ${s.value??s.tag}`);
try {
 for(const [k,x,z] of [['housing',23,-19],['housing',29,-19],['stockpile',-9,-23],['granary',-9,-35],['well',13,-19],['siege_workshop',-24,-24]] as [string,number,number][])ok(await build(k,x,z),k);
 const factory=own().find(b=>b.kind==='siege_workshop')!;const keep=own().find(b=>b.kind==='keep')!;
 if(tower){ok(await build('research_center',24,22),'research center');const center=own().find(b=>b.kind==='research_center')!;ok(await call(db,'startResearch',center.id,'weapons'),'weapons research');await until(()=>db.db.researchState.keepId.find(keep.id)?.weapons===1,'research tower',90000);}
 ok(await call(enemy,'joinGame',game),'enemy');
 for(const kind of (tower ? ['siege_tower'] : process.env.RTS_SIEGE_TRAVERSAL_ONLY ? ['assault_ladder','ram'] : ['assault_ladder','ram','scorpion','pluteus'])){
  await until(()=>db.db.keepPopulation.buildingId.find(keep.id)!.arrived>=(tower?5:3),'crew arrives',60000);
  ok(await call(db,'queueSiege',factory.id,kind),kind);
  await until(()=>Array.from(db.db.armyUnit.iter()).some(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind===kind),kind+' manufactured',65000);
  const u=Array.from(db.db.armyUnit.iter()).find(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind===kind)!;
  ok(await call(db,'setTargets',[u.id],-24-Math.max(0,['assault_ladder','ram','scorpion','pluteus'].indexOf(kind))*6,-42),'clear workshop');
  console.log('PASS manufacture '+kind);
 }
 const engine=(k:string)=>Array.from(db.db.armyUnit.iter()).find(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind===k)!;
 ok(await call(enemy,'buildRotated',-24,-50,wallKind,0),'enemy wall');
 await until(()=>Array.from(db.db.building.iter()).some(b=>b.gameId===game&&b.kind===wallKind),'wall replication');
 const wall=Array.from(db.db.building.iter()).find(b=>b.gameId===game&&b.kind===wallKind)!;
 console.log('Approach',accessKind,engine(accessKind).x,engine(accessKind).z,'target',wall.x,wall.z);
 ok(await call(db,'attackBuilding',[engine(accessKind).id],wall.id),'deploy ladder');
 await until(()=>!!db.db.siegeAccess.unitId.find(engine(accessKind).id),'siege access',tower?100000:35000);
 ok(await call(db,'setTargets',[lord().id],-18,-42),'approach ladder');
 await until(()=>lord().y<2&&Math.hypot(lord().x+18,lord().z+42)<.5,'lord reaches ladder on ground',65000);
 ok(await call(db,'assaultRoof',[lord().id],engine(accessKind).id),'climb');
 await until(()=>lord().y>(tower?10.5:3.9)&&Math.hypot(lord().x-wall.x,lord().z-wall.z)<2,'lord climbs enemy wall',65000);
 await delay(2000);ok(await call(db,'setTargets',[lord().id],-18,-42),'descend');
 await until(()=>lord().y<2&&Math.hypot(lord().x+18,lord().z+42)<.5,'lord returns down ladder',40000);
 console.log('PASS ladder deployment, physical ascent and descent');
 if(!tower){ok(await call(db,'attackBuilding',[engine('ram').id],wall.id),'ram attack');
 await until(()=>!db.db.building.id.find(wall.id)||db.db.building.id.find(wall.id)!.health<.95,'ram damages wall',50000);
 console.log('PASS ram damages enemy fortification');}else{console.log('PASS researched siege tower climbs high wall and returns');}
}finally{db.disconnect();enemy.disconnect()}
