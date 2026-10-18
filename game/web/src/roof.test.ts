// Real-server tests for door/hatch routing, slot reservations, descent and cleanup.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri=process.env.SPACETIMEDB_URI ?? 'http://127.0.0.1:3300';
const database=process.env.SPACETIMEDB_DATABASE ?? 'stronghold-test';
const clients:DbConnection[]=[];
const delay=(ms:number)=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(test:()=>boolean,message:string,timeout=90000) {
  const end=Date.now()+timeout;
  while(!test()) {assert.ok(Date.now()<end,message);await delay(25);}
}
async function connect() {
  return new Promise<DbConnection>((resolve,reject)=>{
    DbConnection.builder().withUri(uri).withModuleName(database).onConnect(db=>{
      clients.push(db);db.subscriptionBuilder().onApplied(()=>resolve(db)).onError(ctx=>reject(ctx.event))
        .subscribe(['SELECT * FROM game','SELECT * FROM participant','SELECT * FROM army_unit','SELECT * FROM building','SELECT * FROM building_rotation','SELECT * FROM tower_occupant']);
    }).onConnectError((_ctx,e)=>reject(e)).build();
  });
}
function status(db:DbConnection,register:(cb:(ctx:ReducerEventContext)=>void)=>void,call:()=>void) {
  return new Promise<string>((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('Reducer timeout')),8000);let done=false;
    register(ctx=>{if(done || ctx.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;
      done=true;clearTimeout(timer);resolve(ctx.event.status.tag);});call();
  });
}
try {
  const a=await connect(),b=await connect();
  assert.equal(await status(a,cb=>a.reducers.onCreateGame(cb),()=>a.reducers.createGame(`Roof test ${Date.now()}`)),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  assert.equal(await status(b,cb=>b.reducers.onJoinGame(cb),()=>b.reducers.joinGame(game)),'Committed');
  const own=()=>Array.from(a.db.armyUnit.iter()).filter(u=>u.gameId===game && u.owner.toHexString()===a.connectionId.toHexString());
  await until(()=>own().length===20,'Own units replicated');
  const initial=own().map(u=>({...u}));
  const ids=initial.map(u=>u.id);
  const build=(x:number,z:number,q:number)=>status(a,cb=>a.reducers.onBuildRotated(cb),()=>a.reducers.buildRotated(x,z,'tower_stairwell',q));
  const roof=(selected:bigint[],id:bigint,x:number,z:number)=>status(a,cb=>a.reducers.onMoveToRoof(cb),()=>a.reducers.moveToRoof(selected,id,x,z));
  const move=(selected:bigint[],x:number,z:number)=>status(a,cb=>a.reducers.onSetTargets(cb),()=>a.reducers.setTargets(selected,x,z));
  const occupants=()=>Array.from(a.db.towerOccupant.iter()).filter(o=>o.gameId===game);
  assert.equal(await build(10,18,0),'Committed');
  await until(()=>Array.from(a.db.building.iter()).some(v=>v.gameId===game),'Tower replicated');
  const tower=Array.from(a.db.building.iter()).find(v=>v.gameId===game)!;
  const arrivals=new Set<bigint>();
  const departureTicks=new Map<unknown,number>();
  let track=true;
  a.db.armyUnit.onUpdate((ctx,old,u)=>{
    if(!track || !ids.includes(u.id))return;
    const teleported=Math.abs(u.y-old.y)>6;
    if(teleported && u.y>7) {
      const door0=Math.hypot(old.x-(tower.x-2.7),old.z-(tower.z+1.04));
      const door1=Math.hypot(old.x-(tower.x-0.99),old.z-(tower.z+2.7));
      assert.ok(Math.min(door0,door1)<=0.551,'Teleport starts at a door');
      assert.ok(Math.hypot(u.x-(tower.x-0.75),u.z-(tower.z+0.9))<=0.151,'Teleport ends at the hatch before roof walk');
      arrivals.add(u.id);
    } else if(teleported) {
      assert.ok(Math.hypot(old.x-(tower.x-0.75),old.z-(tower.z+0.9))<=0.151,'Departure starts at the hatch');
      assert.equal(u.y,0.75);
      const key=ctx.event.tag==='Reducer'?ctx.event.value.timestamp.microsSinceUnixEpoch:ctx.event;
      departureTicks.set(key,(departureTicks.get(key)??0)+1);
    } else {assert.ok(Math.hypot(u.x-old.x,u.y-old.y,u.z-old.z)<0.151,'Walking keeps normal speed');}
    assert.ok(u.y===0.75 || Math.abs(u.y-7.413)<0.002,'No unit walks up the stairs');
  });
  assert.equal(await roof(ids,tower.id,NaN,18),'Failed');
  assert.equal(await roof(ids,tower.id,100,100),'Failed');
  assert.equal(await status(b,cb=>b.reducers.onMoveToRoof(cb),()=>b.reducers.moveToRoof([ids[0]],tower.id,10,18)),'Failed');
  assert.equal(await roof(ids,tower.id,10,18),'Committed');
  await until(()=>occupants().length===20,'All twenty units reserve places, exceeding the former limit of eight');
  assert.equal(new Set(occupants().map(o=>o.slot)).size,20,'Reservations are unique');
  assert.ok(occupants().every(o=>o.slot<25));
  await until(()=>ids.every(id=>{const u=a.db.armyUnit.id.find(id)!;return Math.abs(u.y-7.413)<0.002 && u.x===u.targetX && u.z===u.targetZ;}),'All twenty reach the roof concurrently',35000);
  assert.equal(arrivals.size,20);
  for(const id of ids) {
    const u=a.db.armyUnit.id.find(id)!;
    for(const coordinate of [u.x-tower.x,u.z-tower.z]) assert.ok([-1.4,-0.7,0,0.7,1.4].some(v=>Math.abs(v-coordinate)<0.001),'Slots lie on the 5 by 5 grid');
    for(const other of ids.filter(v=>v<id)) {
      const v=a.db.armyUnit.id.find(other)!;
      assert.ok(Math.hypot(u.x-v.x,u.z-v.z)>=0.5499,'Settled units do not stack');
    }
  }
  await until(()=>Array.from(b.db.towerOccupant.iter()).filter(o=>o.buildingId===tower.id).length===20,'Reservations replicate');
  assert.ok(ids.every(id=>Math.abs(b.db.armyUnit.id.find(id)!.y-7.413)<0.002));
  console.log('PASS: twenty concurrent arrivals, 5 by 5 slots, door-to-hatch jumps and multiplayer.');
  assert.equal(await move(ids,20,30),'Committed');
  await until(()=>occupants().length===0,'All units teleport back to the doors',10000);
  assert.ok(Math.max(...departureTicks.values())>=2,'Multiple units teleport in the same scheduler transaction');
  assert.ok(own().every(u=>u.y===0.75));
  track=false;
  assert.equal(await roof([ids[0]],tower.id,10,18),'Committed');
  assert.equal(await move([ids[0]],22,18),'Committed','A new ground order cancels an approach before teleport');
  await until(()=>!a.db.towerOccupant.unitId.find(ids[0]),'Cancelled approach releases its reservation');
  await until(()=>own().every(u=>u.x>12 || u.z>22),'Army clears the neighbouring plot');
  assert.equal(await build(6,18,0),'Committed');
  assert.equal(await roof([ids[0]],tower.id,10,18),'Committed');
  await until(()=>a.db.towerOccupant.unitId.find(ids[0])?.door===1,'A blocked door selects the second door');
  await until(()=>Math.abs(a.db.armyUnit.id.find(ids[0])!.y-7.413)<0.002,'Second door teleports to the hatch');
  // Every rotation retains its own door/hatch transforms; roof-to-roof transfers descend first.
  for(const q of [1,2,3]) {
    const x=22+q*8,z=18;
    assert.equal(await build(x,z,q),'Committed');
    await until(()=>Array.from(a.db.building.iter()).some(v=>v.gameId===game && v.x===x),'Rotated tower replicated');
    const turned=Array.from(a.db.building.iter()).find(v=>v.gameId===game && v.x===x)!;
    assert.equal(await roof([ids[0]],turned.id,x,z),'Committed');
    await until(()=>a.db.towerOccupant.unitId.find(ids[0])?.buildingId===turned.id && Math.abs(a.db.armyUnit.id.find(ids[0])!.y-7.413)<0.002,'Transfer reaches rotated roof');
  }
  console.log('PASS: concurrent departures, alternate door and rotated roof transfers.');
  assert.equal(await status(b,cb=>b.reducers.onBuild(cb),()=>b.reducers.build(24,30,'granary')),'Committed');
  await until(()=>Array.from(a.db.building.iter()).some(v=>v.gameId===game && v.kind==='granary'),'Enemy replicated');
  const enemy=Array.from(a.db.building.iter()).find(v=>v.gameId===game && v.kind==='granary')!;
  assert.equal(await status(a,cb=>a.reducers.onAttackBuilding(cb),()=>a.reducers.attackBuilding([ids[0]],enemy.id)),'Committed');
  await until(()=>!a.db.towerOccupant.unitId.find(ids[0]),'Attack order teleports back down');
  await until(()=>a.db.building.id.find(enemy.id)!.health<1,'Attack executes on ground');
  // Move the whole ground formation so its centre units are not enclosed by idle peers.
  assert.equal(await roof(ids.slice(1),tower.id,10,18),'Committed');
  await until(()=>ids.slice(1,4).every(id=>Math.abs(a.db.armyUnit.id.find(id)!.y-7.413)<0.002),'Refill original roof');
  assert.equal(await status(a,cb=>a.reducers.onSetFormationTargets(cb),()=>a.reducers.setFormationTargets(ids.slice(1,3),16,26,18,26)),'Committed');
  await until(()=>ids.slice(1,3).every(id=>!a.db.towerOccupant.unitId.find(id)),'Formation order teleports down');
  assert.equal(await status(a,cb=>a.reducers.onRemoveBuilding(cb),()=>a.reducers.removeBuilding(tower.id)),'Committed');
  await until(()=>!a.db.towerOccupant.unitId.find(ids[3]),'Demolition clears roof reservation');
  assert.equal(a.db.armyUnit.id.find(ids[3])!.y,0.75);
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  await until(()=>JSON.parse(strongholdNetwork.snapshot()).ready,'Bridge connected');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Roof bridge ${Date.now()}`}));
  await until(()=>JSON.parse(strongholdNetwork.snapshot()).units.length===20,'Bridge army ready');
  strongholdNetwork.send(JSON.stringify({action:'build',x:10,z:18,building:'tower_stairwell',rotation:1}));
  await until(()=>JSON.parse(strongholdNetwork.snapshot()).buildings.length===1,'Bridge tower ready');
  const state=JSON.parse(strongholdNetwork.snapshot());
  strongholdNetwork.send(JSON.stringify({action:'move_to_roof',unit_ids:state.units.map((u:{id:string})=>u.id),building_id:state.buildings[0].id,x:10,z:18}));
  await until(()=>JSON.parse(strongholdNetwork.snapshot()).roof_occupants.length===20,'Bridge accepts all twenty units');
  await until(()=>JSON.parse(strongholdNetwork.snapshot()).units.every((u:{y:number})=>Math.abs(u.y-7.413)<0.002),'Bridge exposes teleported roof positions');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>JSON.parse(strongholdNetwork.snapshot()).game_id===null,'Bridge leaves');
  assert.equal(await status(a,cb=>a.reducers.onLeaveGame(cb),()=>a.reducers.leaveGame()),'Committed');
  await until(()=>occupants().length===0,'Leave clears all reservations');
  console.log('PASS: concurrent departures, alternate door, rotations, roof transfers, attack/formation orders, demolition, bridge and cleanup.');
} finally { strongholdNetwork.disconnect();for(const client of clients)client.disconnect(); }
