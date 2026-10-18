// Real-server checks for staffing actual production buildings.
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
        .subscribe(['SELECT * FROM game','SELECT * FROM participant','SELECT * FROM army_unit','SELECT * FROM building','SELECT * FROM building_rotation','SELECT * FROM tower_occupant','SELECT * FROM player_resources']);
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
  const a=await connect(), b=await connect();
  const call=(db:DbConnection,name:string,args:unknown[]=[])=>status(db,cb=>(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)](cb),()=>(db.reducers as any)[name](...args));
  assert.equal(await call(a,'createGame',[`Housing ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  assert.equal(await call(b,'joinGame',[game]),'Committed');
  const own=(db=a)=>Array.from(db.db.building.iter()).filter(v=>v.gameId===game&&v.owner.toHexString()===db.connectionId.toHexString());
  const population=(db=a)=>[own(db).filter(v=>['quarry','iron_mine','saw_pit'].includes(v.kind)).length,own(db).filter(v=>v.kind==='housing').length*4];
  assert.deepEqual(population(),[0,0]);
  assert.equal(await call(a,'build',[-1,45,'stockpile']),'Committed','Warehouse requires no peasants');
  assert.equal(await call(a,'build',[31,25,'tower_stairwell']),'Committed','Tower requires no peasants');
  assert.equal(await call(a,'build',[31,45,'granary']),'Committed','Granary requires no peasants');
  assert.equal(await call(a,'build',[71,45,'barracks']),'Committed','Barracks requires no peasants');
  assert.equal(await call(a,'buildWall',[41,81,45,81]),'Committed','Walls require no peasants');
  assert.deepEqual(population(),[0,0]);
  assert.equal(await call(a,'build',[101,45,'quarry']),'Failed','Actual production needs a worker');
  assert.equal(await call(a,'buildRotated',[-101,45,'housing',1]),'Committed','Housing needs no worker or resources');
  await until(()=>population()[1]===4,'Housing replicated');
  for(const [x,z,kind] of [[101,45,'quarry'],[121,45,'iron_mine'],[141,45,'saw_pit'],[161,45,'quarry']] as const) {
    assert.equal(await call(a,'build',[x,z,kind]),'Committed');
  }
  assert.deepEqual(population(),[4,4]);
  assert.equal(await call(a,'build',[181,45,'quarry']),'Failed','Full workforce blocks production');
  assert.equal(await call(b,'build',[201,45,'quarry']),'Failed','Enemy housing provides no workers');
  assert.equal(await call(a,'build',[51,45,'granary']),'Committed','Storage works at capacity');
  assert.equal(await call(a,'buildWall',[47,81,49,81]),'Committed','Wall extension works at capacity');
  assert.deepEqual(population(),[4,4]);
  const producer=own().find(v=>v.kind==='quarry')!;
  assert.equal(await call(a,'removeBuilding',[producer.id]),'Committed');
  await until(()=>population()[0]===3,'Production demolition releases worker');
  assert.equal(await call(a,'build',[181,45,'quarry']),'Committed','Released worker can staff a new producer');
  assert.equal(await call(a,'build',[-101,55,'housing']),'Committed');
  assert.deepEqual(population(),[4,8]);
  assert.equal(await call(a,'build',[201,45,'iron_mine']),'Committed');
  const houses=own().filter(v=>v.kind==='housing');
  assert.equal(await call(a,'removeBuilding',[houses[0].id]),'Committed');
  assert.deepEqual(population(),[5,4],'Housing demolition reduces capacity without deleting businesses');
  assert.equal(await call(a,'build',[221,45,'saw_pit']),'Failed','Over-capacity settlement cannot add production');
  assert.equal(await call(a,'build',[91,65,'granary']),'Committed','Over-capacity settlement can still add storage');
  assert.equal(await call(a,'build',[-101,65,'housing']),'Committed');
  assert.deepEqual(population(),[5,8]);
  assert.equal(await call(a,'build',[221,45,'saw_pit']),'Committed');
  assert.equal(await call(b,'build',[241,45,'housing']),'Committed');
  assert.deepEqual(population(b),[0,4]);
  assert.deepEqual(population(),[6,8],'Population is per player');
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Browser ready');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Housing bridge ${Date.now()}`}));
  await until(()=>snapshot().game_id!==null&&snapshot().resources?.stone===0,'Browser empty start');
  strongholdNetwork.send(JSON.stringify({action:'build',x:-101,z:45,building:'housing'}));
  await until(()=>snapshot().buildings.some((v:any)=>v.owned&&v.kind==='housing'),'Browser receives own housing');
  strongholdNetwork.send(JSON.stringify({action:'build',x:-1,z:45,building:'stockpile'}));
  await until(()=>snapshot().resources?.stone===20,'Browser stockpile gift');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null,'Browser leaves');
  assert.equal(await call(a,'leaveGame'),'Committed');assert.equal(await call(b,'leaveGame'),'Committed');
  console.log('PASS: production-only workforce, free storage/fortifications/barracks, four workers per house, per-player capacity, demolition, over-capacity recovery and browser replication.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
