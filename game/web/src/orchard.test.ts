// Orchard production/food transport integration against a disposable instance.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri=process.env.SPACETIMEDB_URI ?? 'http://127.0.0.1:3300';
const database=process.env.SPACETIMEDB_DATABASE ?? 'stronghold-logistics';
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
        .subscribe(['game','participant','army_unit','building','building_rotation','tower_occupant','player_resources','warehouse_stock','worker','producer','granary_stock'].map(t=>`SELECT * FROM ${t}`));
    }).onConnectError((_ctx,e)=>reject(e)).build();
  });
}
function call(db:DbConnection,name:string,args:unknown[]=[]) {
  return new Promise<string>((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error(`Reducer timeout ${name}`)),8000);let done=false;
    (db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)]((ctx:ReducerEventContext)=>{
      if(done || ctx.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;
      done=true;clearTimeout(timer);resolve(ctx.event.status.tag);
    });(db.reducers as any)[name](...args);
  });
}
try {
  const a=await connect(), observer=await connect();
  assert.equal(await call(a,'createGame',[`Orchard ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  const buildings=()=>Array.from(a.db.building.iter()).filter(s=>s.gameId===game);
  const farms=()=>buildings().filter(b=>b.kind==='apple_farm');
  const producers=()=>Array.from(a.db.producer.iter()).filter(p=>p.gameId===game);
  const workers=()=>Array.from(a.db.worker.iter()).filter(w=>w.gameId===game);
  const stocks=()=>Array.from(a.db.granaryStock.iter()).filter(s=>s.gameId===game);
  const food=()=>stocks().reduce((n,s)=>[n[0]+s.apples,n[1]+s.bread,n[2]+s.cheese,n[3]+s.meat],[0,0,0,0]);
  assert.equal(await call(a,'build',[21,45,'apple_farm']),'Failed','Orchard needs one free peasant');
  assert.equal(await call(a,'build',[-101,45,'housing']),'Committed');
  assert.equal(await call(a,'build',[-1,65,'stockpile']),'Committed');
  assert.equal(await call(a,'buildRotated',[21,45,'apple_farm',1]),'Committed');
  await until(()=>farms().length===1&&workers().length===1,'Orchard and worker replicated');
  const farm=farms()[0];
  assert.equal(producers()[0].resource,3,'Orchard produces apples');
  assert.equal(workers()[0].resource,3,'Transporter carries apples');
  assert.equal(await call(a,'build',[farm.x+6,farm.z,'granary']),'Failed','Farm reserves paddock as well as barn');
  await until(()=>producers()[0].buffer>=2,'Orchard produces without a granary');
  assert.deepEqual(food(),[0,0,0,0]);assert.equal(workers()[0].cargo,0,'Stockpiles cannot accept apples');
  assert.equal(await call(a,'build',[-1,45,'granary']),'Committed');
  await until(()=>stocks().length===1,'Granary receives starting food');
  const granary=stocks()[0];
  assert.deepEqual(food(),[5,5,5,5]);
  await until(()=>workers()[0].cargo>0,'Worker picks up apples');
  await until(()=>food()[0]>5,'Worker physically delivers apples');
  assert.equal(food()[2],5);assert.equal(food()[1],5);assert.equal(food()[3],5);
  await until(()=>Array.from(observer.db.granaryStock.iter()).some(s=>s.buildingId===granary.buildingId&&s.apples>5),'Observers see delivered apples');
  await until(()=>food()[0]===24,'Partial last delivery fills the granary apples capacity',120000);
  await delay(4000);
  assert.equal(food()[0],24,'Full granary cannot overflow');
  assert(producers()[0].buffer<=20);assert(workers()[0].cargo<=5);
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Browser ready');
  strongholdNetwork.send(JSON.stringify({action:'join',game_id:game.toString()}));
  await until(()=>snapshot().buildings.some((b:any)=>b.id===farm.id.toString()&&b.kind==='apple_farm')&&snapshot().granaries.some((s:any)=>s.building_id===granary.buildingId.toString()&&s.apples===24),'Browser receives farm and physical food');
  assert.equal(snapshot().resources.apples,0,'Observers do not own the apples');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null,'Browser leaves');
  assert.equal(await call(a,'removeBuilding',[granary.buildingId]),'Committed');
  await until(()=>stocks().length===0,'Destroyed granary loses food');
  assert.equal(await call(a,'build',[-1,45,'granary']),'Committed');
  await until(()=>stocks().length===1,'Replacement granary');
  assert.deepEqual(food(),[0,0,0,0],'Replacement has no repeated starting gift');
  await until(()=>food()[0]>0,'Orchard transporter resumes delivery to replacement storage');
  assert.equal(food()[2],0);assert.equal(food()[1],0);assert.equal(food()[3],0);
  const resources=a.db.playerResources.connectionId.find(a.connectionId)!;
  assert.deepEqual([resources.wood,resources.stone,resources.iron],[20,20,20],'Food production preserves construction balances');
  assert.equal(await call(a,'removeBuilding',[farm.id]),'Committed');
  await until(()=>workers().length===0&&producers().length===0,'Farm demolition removes production worker');
  assert.equal(await call(a,'leaveGame'),'Committed');
  console.log('PASS: orchard peasant requirement, orchard occupancy, apples production and real granary transport, capacity/partial cargo, observers/browser, storage replacement and demolition cleanup.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
