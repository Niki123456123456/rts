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
  const call=(db:DbConnection,name:string,args:unknown[])=>status(db,cb=>(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)](cb),()=>(db.reducers as any)[name](...args));
  assert.equal(await call(a,'createGame',[`Stone walls ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  assert.equal(await call(b,'joinGame',[game]),'Committed');
  const own=()=>Array.from(a.db.armyUnit.iter()).filter(u=>u.gameId===game && u.owner.toHexString()===a.connectionId.toHexString());
  await until(()=>own().length===20,'Army replicated');
  const ids=own().map(u=>u.id);
  const buildings=()=>Array.from(a.db.building.iter()).filter(v=>v.gameId===game);
  const walls=()=>buildings().filter(v=>v.kind==='stone_wall');
  const wall=(x:number,z:number)=>walls().find(v=>v.x===x&&v.z===z)!;
  const roof=(selected:bigint[],row:ReturnType<typeof wall>)=>call(a,'moveToRoof',[selected,row.id,row.x,row.z]);
  const settled=(selected:bigint[])=>selected.every(id=>{const u=a.db.armyUnit.id.find(id)!;return u.y>7&&u.x===u.targetX&&u.z===u.targetZ;});
  assert.equal(await call(a,'buildWall',[13,19,13,19]),'Committed');
  await until(()=>walls().length===1,'Single segment replicated');
  assert.equal(await roof(ids,wall(13,19)),'Failed','Wall without tower cannot be entered');
  assert.equal(await call(a,'build',[10,18,'tower_stairwell']),'Committed');
  await until(()=>buildings().some(v=>v.kind==='tower_stairwell'),'Access tower replicated');
  const tower=buildings().find(v=>v.kind==='tower_stairwell')!;
  let arrivals=0;
  a.db.armyUnit.onUpdate((_ctx,old,u)=>{
    if(!ids.includes(u.id))return;
    if(old.y===0.75&&u.y>1&&walls().length<=3) {
      const doors=[[-2.7,1.04],[-0.99,2.7]];
      assert.ok(doors.some(([x,z])=>Math.hypot(old.x-tower.x-x,old.z-tower.z-z)<=0.551),'Entry starts at tower door');
      assert.ok(Math.hypot(u.x-tower.x+0.75,u.z-tower.z-0.9)<=0.151,'Entry emerges at tower hatch');
      arrivals++;
    }
    if(u.y===0.75) assert.ok(!walls().some(v=>Math.abs(u.x-v.x)<1.269&&Math.abs(u.z-v.z)<1.269),'Ground units never enter wall obstacles');
  });
  assert.equal(await roof(ids,wall(13,19)),'Committed');
  await until(()=>own().filter(u=>u.y>7&&u.x===u.targetX&&u.z===u.targetZ).length===9,'One 2x2 m segment fits exactly nine units');
  assert.equal(Array.from(a.db.towerOccupant.iter()).filter(o=>o.gameId===game).length,9);
  assert.equal(await call(a,'buildWall',[13,19,17,19]),'Committed','Existing own segment reused when extending');
  await until(()=>walls().length===3,'Inclusive line creates three side-connected segments');
  assert.equal(await roof(ids,wall(17,19)),'Committed');
  await until(()=>settled(ids),'Twenty units use the area of the entire connected wall');
  assert.ok(arrivals>=20,'All ground entrants used the tower');
  assert.equal(new Set(ids.map(id=>{const o=a.db.towerOccupant.unitId.find(id)!;return `${o.buildingId}:${o.slot}`;})).size,20);
  for(const u of own())for(const v of own())if(u.id!==v.id)assert.ok(Math.hypot(u.x-v.x,u.z-v.z)>0.549,'Distinct safe roof positions');
  await until(()=>ids.every(id=>b.db.armyUnit.id.find(id)!.y>7),'Wall positions replicate to other player');
  const enemy=Array.from(b.db.armyUnit.iter()).find(u=>u.owner.toHexString()===b.connectionId.toHexString())!;
  assert.equal(await call(b,'moveToRoof',[[enemy.id],wall(17,19).id,17,19]),'Failed','Enemy cannot use this wall network');
  const far=own().find(u=>a.db.towerOccupant.unitId.find(u.id)!.buildingId===wall(17,19).id)!;
  assert.equal(await roof([far.id],wall(13,19)),'Committed','Surface transfer across wall segments');
  await until(()=>settled([far.id])&&a.db.towerOccupant.unitId.find(far.id)!.buildingId===wall(13,19).id,'Walks across connected wall');
  assert.equal(await roof([far.id],wall(17,19)),'Committed');
  await until(()=>settled([far.id])&&a.db.towerOccupant.unitId.find(far.id)!.buildingId===wall(17,19).id,'Returns to far wall');
  assert.equal(await call(a,'moveToRoof',[[far.id],tower.id,tower.x,tower.z]),'Committed','Wall to tower-roof transfer');
  await until(()=>settled([far.id])&&a.db.towerOccupant.unitId.find(far.id)!.buildingId===tower.id,'Unit reaches original tower roof');
  assert.equal(await roof([far.id],wall(17,19)),'Committed','Tower roof to wall transfer');
  await until(()=>settled([far.id])&&a.db.towerOccupant.unitId.find(far.id)!.buildingId===wall(17,19).id,'Unit returns to wall through tower');
  const bridge=wall(15,19);
  assert.equal(await call(a,'removeBuilding',[bridge.id]),'Committed');
  await until(()=>!a.db.building.id.find(bridge.id),'Gap replicated');
  assert.equal(await roof([far.id],wall(17,19)),'Failed','Disconnected wall has no tower access');
  assert.equal(await call(a,'setTargets',[[far.id],25,19]),'Failed','No descent through missing wall segment');
  const evacuated=own().filter(u=>u.y===0.75).map(u=>u.id);
  assert.equal(await call(a,'setTargets',[evacuated,25,30]),'Committed');
  await until(()=>evacuated.every(id=>a.db.armyUnit.id.find(id)!.x>20),'Evacuated units clear the construction plot');
  assert.equal(await call(a,'buildWall',[15,19,15,19]),'Committed');
  await until(()=>!!wall(15,19),'Bridge rebuilt');
  assert.equal(await call(a,'setTargets',[[far.id],25,19]),'Committed');
  await until(()=>{const u=a.db.armyUnit.id.find(far.id)!;return u.y===0.75&&Math.hypot(u.x-25,u.z-19)<0.01;},'Unit walks to tower, teleports to door, and navigates around ground wall');
  console.log('PASS: capacity, tower entry, connected wall transfers and gap rejection.');
  const before=walls().length;
  assert.equal(await call(a,'build',[35,35,'granary']),'Committed');
  assert.equal(await call(a,'buildWall',[29,35,41,35]),'Failed','Blocked line rejects atomically');
  assert.equal(walls().length,before,'No partial wall remains after rejected build');
  assert.equal(await call(a,'buildWall',[45,19,51,25]),'Committed');
  await until(()=>walls().length===before+7,'Diagonal line uses seven gap-free raster steps');
  const diagonal=walls().filter(v=>v.x>=45&&v.x<=51&&v.z>=19&&v.z<=25);
  assert.equal(diagonal.length,7);
  assert.equal(await call(a,'build',[60,18,'tower_stairwell']),'Committed');
  assert.equal(await call(a,'buildWall',[63,21,63,21]),'Committed');
  await until(()=>!!wall(63,21),'Corner wall replicated');
  assert.equal(await roof([far.id],wall(63,21)),'Failed','Corner-only tower contact cannot be walked');
  for(const [kind,x,wallX] of [['tower_lookout',10,13],['tower_square',31,35]] as const) {
    assert.equal(await call(a,'build',[x,48,kind]),'Committed');
    const z=49;
    assert.equal(await call(a,'buildWall',[wallX,z,wallX+2,z]),'Committed');
    await until(()=>!!wall(wallX+2,z),'Higher tower wall replicated');
    assert.equal(await roof([far.id],wall(wallX+2,z)),'Committed',`${kind} accesses the lower wall through its internal hatch connection`);
    await until(()=>settled([far.id])&&Math.abs(a.db.armyUnit.id.find(far.id)!.y-7.413)<0.002,'Wall remains the stair-tower height');
    assert.equal(await call(a,'setTargets',[[far.id],25,35]),'Committed');
    await until(()=>a.db.armyUnit.id.find(far.id)!.y===0.75,'Higher tower exit reaches ground');
  }
  const walker=own().find(u=>u.id!==far.id&&u.y>7)!;
  assert.equal(await call(a,'setTargets',[[walker.id],25,19]),'Committed');
  await delay(100);
  assert.equal(await roof([walker.id],wall(17,19)),'Committed','A new wall order replaces an in-progress descent');
  await until(()=>settled([walker.id])&&a.db.towerOccupant.unitId.find(walker.id)!.buildingId===wall(17,19).id,'Interrupted descent finishes on wall instead of retaining a ground order');
  assert.equal(await call(a,'buildWall',[17,19,25,19]),'Committed');
  await until(()=>!!wall(25,19),'Long extension replicated');
  assert.equal(await roof([walker.id],wall(25,19)),'Committed');
  await until(()=>{const u=a.db.armyUnit.id.find(walker.id)!;return u.x>18&&u.x<21;},'Unit walking along extension');
  assert.equal(await call(a,'removeBuilding',[wall(21,19).id]),'Committed');
  await until(()=>a.db.armyUnit.id.find(walker.id)!.y===0.75&&!a.db.towerOccupant.unitId.find(walker.id),'Removing a route segment cancels crossing travel and evacuates the walker');
  const removedWall=wall(17,19).id;
  assert.equal(await call(a,'removeBuilding',[removedWall]),'Committed');
  await until(()=>!Array.from(a.db.towerOccupant.iter()).some(o=>o.buildingId===removedWall),'Demolition releases occupied wall slots');
  assert.equal(await call(a,'leaveGame',[]),'Committed');
  await until(()=>!Array.from(a.db.towerOccupant.iter()).some(o=>o.gameId===game),'Wall reservations clean up');
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Browser bridge ready');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Wall bridge ${Date.now()}`}));
  await until(()=>snapshot().units.length===20,'Browser army replicated');
  strongholdNetwork.send(JSON.stringify({action:'build',building:'tower_stairwell',x:10,z:18}));
  await until(()=>snapshot().buildings.length===1,'Browser access tower');
  strongholdNetwork.send(JSON.stringify({action:'build_wall',start_x:13,start_z:19,end_x:17,end_z:19}));
  await until(()=>snapshot().buildings.length===4,'Browser wall action constructs inclusive raster line');
  const browserWall=snapshot().buildings.find((v:any)=>v.kind==='stone_wall'&&v.x===17);
  strongholdNetwork.send(JSON.stringify({action:'move_to_roof',unit_ids:snapshot().units.map((u:any)=>u.id),building_id:browserWall.id,x:17,z:19}));
  await until(()=>snapshot().units.every((u:any)=>Math.abs(u.y-7.413)<0.002),'Browser bridge places units on wall');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null,'Browser wall cleanup');
  console.log('PASS: atomic raster wall lines, nine slots per segment, connected-area capacity, all three tower entrances/exits, wall transfers, gaps, navigation, ownership and multiplayer.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
