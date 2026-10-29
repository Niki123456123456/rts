// Uses an isolated local database, never an existing player's world.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
const clients: DbConnection[] = [];
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
async function until(test: () => boolean, label: string, milliseconds = 15000) {
  const deadline = Date.now() + milliseconds;
  while (!test()) { assert.ok(Date.now() < deadline, label); await delay(30); }
}
async function connect() {
  return new Promise<DbConnection>((resolve, reject) => {
    DbConnection.builder().withUri(process.env.SPACETIMEDB_URI ?? 'http://127.0.0.1:18766')
      .withModuleName(process.env.SPACETIMEDB_DATABASE ?? 'stronghold-terrain-test')
      .onConnect(db => {
        clients.push(db);
        db.subscriptionBuilder().onApplied(() => resolve(db)).onError(ctx => reject(ctx.event))
          .subscribe(['SELECT * FROM game','SELECT * FROM participant','SELECT * FROM building',
            'SELECT * FROM keep_population','SELECT * FROM army_unit','SELECT * FROM unit_profile',
            'SELECT * FROM tower_occupant','SELECT * FROM worker']);
      }).onConnectError((_ctx, error) => reject(error)).build();
    setTimeout(() => reject(new Error('Connection timeout')),10000).unref();
  });
}
function status(db: DbConnection, register: (cb: (ctx: ReducerEventContext) => void) => void, call: () => void) {
  return new Promise<ReducerEventContext['event']['status']>((resolve,reject) => {
    let done = false;
    const timer = setTimeout(()=>reject(new Error('Reducer timeout')),10000);
    register(ctx=>{
      if(done || ctx.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString()) return;
      done=true;clearTimeout(timer);resolve(ctx.event.status);
    });call();
  });
}
try {
  const a=await connect(),b=await connect();
  assert.equal((await status(a,cb=>a.reducers.onCreateGame(cb),()=>a.reducers.createGame(`Keep test ${Date.now()}`))).tag,'Committed');
  const membership=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!;
  const game=membership.gameId;
  const buildings=()=>Array.from(a.db.building.iter()).filter(row=>row.gameId===game);
  const build=(kind:string,x:number,z:number,rotation=0)=>status(a,cb=>a.reducers.onBuildRotated(cb),()=>a.reducers.buildRotated(x,z,kind,rotation));
  const lord=()=>Array.from(a.db.armyUnit.iter()).find(u=>u.gameId===game && u.owner.toHexString()===a.connectionId.toHexString() && u.number===1)!;
  await until(()=>buildings().some(b=>b.kind==='keep'),'keep generated when game is created');
  const keep=buildings().find(b=>b.kind==='keep')!;
  assert.equal(buildings().filter(b=>b.kind==='keep').length,1);
  assert.equal((await status(a,cb=>a.reducers.onJoinGame(cb),()=>a.reducers.joinGame(game))).tag,'Committed');
  assert.equal(buildings().filter(b=>b.kind==='keep').length,1,'joining again does not duplicate the keep');
  await until(()=>!!a.db.towerOccupant.unitId.find(lord().id),'lord starts on terrace');
  assert.ok(Math.abs(lord().y-6.15)<.02,'lord stands at 5.4 m terrace height');
  assert.equal((await build('keep',-60,0)).tag,'Failed','one keep per player');
  assert.equal((await build('apple_farm',0,24)).tag,'Committed','empty workplaces can be built before immigration');
  assert.equal((await build('granary',-9,-23)).tag,'Committed','food attracts new peasants');
  assert.equal((await status(b,cb=>b.reducers.onJoinGame(cb),()=>b.reducers.joinGame(game))).tag,'Committed');
  await until(()=>buildings().filter(row=>row.kind==='keep').length===2,'second player receives a separate keep');
  const secondKeep=buildings().find(row=>row.kind==='keep'&&row.id!==keep.id)!;
  assert.notEqual(secondKeep.owner.toHexString(),keep.owner.toHexString());
  assert.ok(Math.abs(secondKeep.x-keep.x)>=20 || Math.abs(secondKeep.z-keep.z)>=28,'starting plots do not overlap');
  await until(()=>!!b.db.keepPopulation.buildingId.find(keep.id),'other players see migration');
  assert.equal((await status(b,cb=>b.reducers.onSetTargets(cb),()=>b.reducers.setTargets([lord().id],0,30))).tag,'Failed','other players cannot control the lord');
  assert.equal((await status(a,cb=>a.reducers.onSetTargets(cb),()=>a.reducers.setTargets([lord().id],keep.x+12,keep.z+18))).tag,'Committed');
  await until(()=>lord().y>1 && lord().y<5,'lord visibly descends staircase');
  await until(()=>!a.db.towerOccupant.unitId.find(lord().id),'lord reaches ground');
  assert.ok(Math.abs(lord().y-.75)<.02);
  await until(()=>a.db.keepPopulation.buildingId.find(keep.id)!.arrived>=1,'first immigrant arrives');
  const farm=buildings().find(row=>row.kind==='apple_farm')!;
  await until(()=>Array.from(a.db.worker.iter()).some(w=>w.sourceBuilding===farm.id),'immigrant automatically takes the open production job');
  await until(()=>Array.from(a.db.worker.iter()).some(w=>w.gameId===game),'production worker exists');
  assert.equal((await build('housing',-42,20)).tag,'Committed');
  assert.equal((await status(a,cb=>a.reducers.onMoveToRoof(cb),()=>a.reducers.moveToRoof([lord().id],keep.id,keep.x,keep.z+2))).tag,'Committed');
  await until(()=>lord().y>1 && lord().y<5,'lord visibly climbs staircase');
  await until(()=>Math.abs(lord().y-6.15)<.02,'lord returns to terrace',20000);
  // Every controllable unit type uses the same physical staircase.
  assert.equal((await build('barracks',keep.x+40,keep.z-12)).tag,'Committed');
  const barracks=buildings().find(row=>row.kind==='barracks'&&row.owner.toHexString()===a.connectionId.toHexString())!;
  for(const kind of ['armed_peasant','archer']) {
    assert.equal((await status(a,cb=>a.reducers.onRecruitUnit(cb),()=>a.reducers.recruitUnit(barracks.id,kind))).tag,'Committed');
    const profile=Array.from(a.db.unitProfile.iter()).find(p=>p.gameId===game&&p.kind===kind&&a.db.armyUnit.id.find(p.unitId)?.owner.toHexString()===a.connectionId.toHexString())!;
    const soldier=()=>a.db.armyUnit.id.find(profile.unitId)!;
    assert.equal((await status(a,cb=>a.reducers.onMoveToRoof(cb),()=>a.reducers.moveToRoof([profile.unitId],keep.id,keep.x,keep.z))).tag,'Committed');
    await until(()=>soldier().y>1&&soldier().y<5,`${kind} climbs actual stairs`,30000);
    await until(()=>Math.abs(soldier().y-6.15)<.02,`${kind} reaches terrace`,30000);
    const departure=await status(a,cb=>a.reducers.onSetTargets(cb),()=>a.reducers.setTargets([profile.unitId],keep.x+12,keep.z+18+(kind==='archer'?3:0)));
    assert.equal(departure.tag,'Committed',`${kind} descent: ${departure.tag==='Failed'?departure.value:departure.tag}`);
    await until(()=>soldier().y>1&&soldier().y<5,`${kind} descends actual stairs`,30000);
    await until(()=>!a.db.towerOccupant.unitId.find(profile.unitId),`${kind} reaches ground`,30000);
  }
  await until(()=>a.db.keepPopulation.buildingId.find(keep.id)!.arrived>=2,'further job seekers arrive');
  await until(()=>b.db.keepPopulation.buildingId.find(keep.id)?.arrived===a.db.keepPopulation.buildingId.find(keep.id)?.arrived,'migration replicated');
  assert.equal((await status(a,cb=>a.reducers.onRemoveBuilding(cb),()=>a.reducers.removeBuilding(keep.id))).tag,'Committed');
  await until(()=>!a.db.keepPopulation.buildingId.find(keep.id),'demolition cleans up immigration');
  await until(()=>!a.db.towerOccupant.unitId.find(lord().id),'lord evacuated on demolition');
  assert.equal((await status(a,cb=>a.reducers.onJoinGame(cb),()=>a.reducers.joinGame(game))).tag,'Committed');
  assert.equal(buildings().filter(row=>row.owner.toHexString()===a.connectionId.toHexString()&&row.kind==='keep').length,0,'demolition does not grant another free keep');
  assert.equal((await status(a,cb=>a.reducers.onCreateGame(cb),()=>a.reducers.createGame(`Keep switch ${Date.now()}`))).tag,'Committed');
  const nextMembership=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!;
  assert.notEqual(nextMembership.gameId,game);
  await until(()=>Array.from(a.db.building.iter()).some(row=>row.gameId===nextMembership.gameId&&row.owner.toHexString()===a.connectionId.toHexString()&&row.kind==='keep'),'new game grants a new starting keep');
  console.log('PASS: automatic keep on creation and player join, separate plots, idempotent join, lord, stairs, immigration, employment, multiplayer, one-time grant and new-game reset.');
} finally { for(const db of clients) db.disconnect(); }
