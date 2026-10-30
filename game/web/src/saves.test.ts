import assert from 'node:assert/strict';
import {DbConnection} from './module_bindings';
import {world,status,until,connect,delay} from './test-world';
const ordered=(rows:unknown[][])=>rows.sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b)));
const w=await world();let db=w.db;let second:DbConnection|undefined;let again:DbConnection|undefined;let newcomer:DbConnection|undefined;
try {
 assert.equal(w.lord().health,5,'lord has five times the normal health');
 await delay(1500);
 const animals=()=>Array.from(db.db.wildlife.iter()).filter(a=>a.gameId===w.game);
 assert.equal(animals().filter(a=>a.species>=3).length,7);
 const palace=w.own().find(b=>b.kind==='keep')!;
 for(const a of animals().filter(a=>a.species>=3))assert.ok(Math.hypot(a.x-palace.x,a.z-palace.z)>150,'predators begin far from palace');
 assert.equal((await w.build('stockpile',-9,-43)).tag,'Committed');
 const savedWood=db.db.playerResources.connectionId.find(db.connectionId)!.wood;
 const buildings=ordered(w.own().map(b=>[b.kind,b.x,b.z,b.health]));
 second=await connect();await status(second,c=>second!.reducers.onJoinGame(c),()=>second!.reducers.joinGame(w.game));
 const otherBuildings=Array.from(second.db.building.iter()).filter(b=>b.owner.toHexString()===second!.connectionId.toHexString()).map(b=>[b.kind,b.x,b.z]);
 const otherToken=second.token!;assert.ok(otherToken);
 assert.equal((await status(db,c=>db.reducers.onSaveGame(c),()=>db.reducers.saveGame('Save verification'))).tag,'Committed');
 const save=Array.from(db.db.saveCatalog.iter()).find(s=>s.owner.toHexString()===db.identity!.toHexString())!;assert.ok(save);
 assert.equal((await status(second,c=>second!.reducers.onLoadGame(c),()=>second!.reducers.loadGame(save.id))).tag,'Failed','other identity cannot load private save');
 assert.equal((await w.build('housing',-17,17)).tag,'Committed');
 assert.equal((await status(db,c=>db.reducers.onLoadGame(c),()=>db.reducers.loadGame(save.id))).tag,'Committed');
 const loaded=Array.from(db.db.participant.iter()).find(p=>p.connectionId.toHexString()===db.connectionId.toHexString())!.gameId;assert.notEqual(loaded,w.game);
 for(const table of [db.db.armyUnit,db.db.building,db.db.wildlife]){const rows=Array.from(table.iter() as Iterable<{id:bigint}>);assert.equal(new Set(rows.map(r=>r.id)).size,rows.length,'no stale versions after loading');}
 assert.deepEqual(ordered(Array.from(db.db.building.iter()).filter(b=>b.gameId===loaded&&b.owner.toHexString()===db.connectionId.toHexString()).map(b=>[b.kind,b.x,b.z,b.health])),buildings);
 assert.equal(db.db.playerResources.connectionId.find(db.connectionId)!.wood,savedWood);
 newcomer=await connect();assert.equal((await status(newcomer,c=>newcomer!.reducers.onJoinGame(c),()=>newcomer!.reducers.joinGame(loaded))).tag,'Committed');
 const colors=Array.from(newcomer.db.participant.iter()).filter(p=>p.gameId===loaded).map(p=>p.color);assert.ok(!colors.includes(1),'offline co-player color remains reserved');
 assert.equal((await status(second,c=>second!.reducers.onJoinGame(c),()=>second!.reducers.joinGame(loaded))).tag,'Committed');
 assert.deepEqual(Array.from(second.db.building.iter()).filter(b=>b.gameId===loaded&&b.owner.toHexString()===second!.connectionId.toHexString()).map(b=>[b.kind,b.x,b.z]),otherBuildings,'co-player reclaims exact settlement');
 const token=db.token!;assert.ok(token);db.disconnect();await delay(400);again=await connect(token);db=again;
 assert.ok(Array.from(db.db.saveCatalog.iter()).some(s=>s.id===save.id&&s.owner.toHexString()===db.identity!.toHexString()),'save survives reconnect');
 assert.equal((await status(db,c=>db.reducers.onLoadGame(c),()=>db.reducers.loadGame(save.id))).tag,'Committed','reload after browser reconnect');
 console.log('PASS: remote sparse predators, fivefold lord HP, private persistent save, material/building restore, co-player ownership reclaim and reconnect/load.');
} finally {db.disconnect();second?.disconnect();again?.disconnect();newcomer?.disconnect();}
