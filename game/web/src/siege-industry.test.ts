import assert from 'node:assert/strict';
import {world,connect,status,until,delay} from './test-world';
const {db,game,own,build}=await world();const other=await connect();
const call=(d:any,name:string,...args:any[])=>status(d,c=>d.reducers[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>d.reducers[name](...args));
const ok=(s:any,label:string)=>assert.equal(s.tag,'Committed',`${label}: ${s.value??s.tag}`);
const engines=()=>Array.from(db.db.armyUnit.iter()).filter(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind==='ballista');
try {
 ok(await build('housing',23,-19),'housing');ok(await build('siege_workshop',-22,-24),'fabrica');ok(await build('harbor',88,-21,3),'shore harbor');assert.equal((await build('harbor',35,35)).tag,'Failed','harbor cannot be inland');
 const factory=own().find(b=>b.kind==='siege_workshop')!,harbor=own().find(b=>b.kind==='harbor')!;
 const gold=db.db.treasury.connectionId.find(db.connectionId)!.gold;ok(await call(db,'marketTrade',0,1,true),'buy using harbor');assert.ok(db.db.treasury.connectionId.find(db.connectionId)!.gold<gold);
 ok(await call(other,'joinGame',game),'second player joins');assert.equal((await call(other,'queueSiege',factory.id,'ballista')).tag,'Failed','cannot use another player factory');
 await until(()=>db.db.keepPopulation.buildingId.find(own().find(b=>b.kind==='keep')!.id)!.arrived>=3,'worker and two free crew arrive',25000);
 const before=db.db.playerResources.connectionId.find(db.connectionId)!;ok(await call(db,'queueSiege',factory.id,'ballista'),'ballista paid order');assert.equal(before.wood-db.db.playerResources.connectionId.find(db.connectionId)!.wood,20);assert.equal(before.iron-db.db.playerResources.connectionId.find(db.connectionId)!.iron,8);assert.equal((await call(db,'queueSiege',factory.id,'onager')).tag,'Failed','one order at a time');
 ok(await call(db,'unassignPeasant',factory.id),'close workshop');const progress=db.db.factoryOrder.buildingId.find(factory.id)!.progress;await delay(3000);assert.equal(db.db.factoryOrder.buildingId.find(factory.id)!.progress,progress);ok(await call(db,'assignPeasant',factory.id),'reopen workshop');
 await until(()=>engines().length===1,'paid ballista finishes manufacture',75000);console.log('PASS ballista manufacture');const ballista=engines()[0];
 // Put a low enemy wall in the open firing lane west of the workshop.
 let target:any;for(const [dx,dz] of [[-18,0],[0,18],[-18,12],[18,15]]){const s=await call(other,'buildRotated',ballista.x+dx,ballista.z+dz,'stone_wall_low',0);if(s.tag==='Committed'){target=Array.from(other.db.building.iter()).find((b:any)=>b.gameId===game&&b.owner.toHexString()===other.connectionId.toHexString()&&b.kind==='stone_wall_low');break;}}assert.ok(target,'enemy wall placed');
 let sawBolt=false;db.db.projectileStyle.onInsert((_c,p)=>{if(p.gameId===game&&p.kind==='ballista')sawBolt=true});ok(await call(db,'attackBuilding',[ballista.id],target.id),'fire at enemy wall');
 await until(()=>sawBolt,'ballista fires a real projectile',35000);await until(()=>!db.db.building.id.find(target.id)||db.db.building.id.find(target.id)!.health<1,'bolt damages the enemy wall',20000);console.log('PASS ballista projectile and damage');
 await until(()=>db.db.keepPopulation.buildingId.find(own().find(b=>b.kind==='keep')!.id)!.arrived>=3,'two more crew arrive',25000);ok(await call(db,'queueSiege',factory.id,'onager'),'onager order');
 ok(await call(db,'saveGame',`Factory ${Date.now()}`),'save unfinished order');const slot=Array.from(db.db.saveCatalog.iter()).sort((a,b)=>Number(b.id-a.id))[0];ok(await call(db,'loadGame',slot.id),'restore unfinished order');const newGame=Array.from(db.db.participant.iter()).find(p=>p.connectionId.toHexString()===db.connectionId.toHexString())!.gameId;assert.ok(Array.from(db.db.factoryOrder.iter()).some(o=>o.gameId===newGame&&o.kind==='onager'));
 await until(()=>Array.from(db.db.armyUnit.iter()).some(u=>u.gameId===newGame&&db.db.unitProfile.unitId.find(u.id)?.kind==='onager'),'saved onager order completes once',80000);
 console.log('PASS: harbor shore-only trade, paid and staffed Fabrica, crew costs and ownership, closed factory pauses, real ballista damage, queued onager survives save/load and completes.');
}finally{db.disconnect();other.disconnect()}
