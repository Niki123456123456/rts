import assert from 'node:assert/strict';
import {world,connect,status,until,delay} from './test-world';
const {db,game,own,build,lord}=await world(),enemy=await connect();const call=(d:any,n:string,...args:any[])=>status(d,c=>d.reducers[`on${n[0].toUpperCase()+n.slice(1)}`](c),()=>d.reducers[n](...args));const ok=(s:any,l:string)=>assert.equal(s.tag,'Committed',`${l}: ${s.value??s.tag}`);
try{
 ok(await build('hospital',-24,-24),'hospital');ok(await call(enemy,'joinGame',game),'enemy joins');
 const enemyLord=()=>Array.from(enemy.db.armyUnit.iter()).find(u=>u.gameId===game&&u.owner.toHexString()===enemy.connectionId.toHexString()&&enemy.db.unitProfile.unitId.find(u.id)?.kind==='lord')!;
 ok(await call(db,'setTargets',[lord().id],-16,-24),'lord approaches hospital');await until(()=>Math.hypot(lord().x+16,lord().z+24)<.4,'lord at hospital approach',50000);
 await until(()=>Array.from(db.db.worker.iter()).some(w=>w.sourceBuilding===own().find(b=>b.kind==='hospital')!.id),'sanitarian staffed',15000);
 ok(await call(enemy,'setTargets',[enemyLord().id],-15,-24),'enemy attacks');await until(()=>lord().health<4.9,'lord takes real damage',65000);await until(()=>Array.from(db.db.attackAlert.iter()).some(a=>a.gameId===game&&a.owner.toHexString()===db.connectionId.toHexString()),'own attack position warning published',5000);
 const alert=Array.from(db.db.attackAlert.iter()).find(a=>a.gameId===game&&a.owner.toHexString()===db.connectionId.toHexString())!;assert.ok(Math.hypot(alert.x-lord().x,alert.z-lord().z)<8);
 ok(await call(enemy,'leaveGame'),'end attack');const wounded=lord().health;await until(()=>lord().health>wounded+.08,'hospital heals after combat',20000);assert.ok(lord().health<=5);await until(()=>!Array.from(db.db.attackAlert.iter()).some(a=>a.gameId===game),'warning expires when no attack continues',7000);
 console.log('PASS: hospital automatically staffed, actual combat damage creates own minimap warning, alert expires after combat, nearby hospital restores health without exceeding maximum.');
}finally{db.disconnect();enemy.disconnect()}
