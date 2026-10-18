// Integration test against a disposable SpacetimeDB instance.
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
        .subscribe(['game','participant','army_unit','building','building_rotation','tower_occupant','player_resources','warehouse_stock','worker','producer'].map(t=>`SELECT * FROM ${t}`));
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
  assert.equal(await call(a,'createGame',[`Logistics ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  const balance=()=>a.db.playerResources.connectionId.find(a.connectionId)!;
  const warehouses=()=>Array.from(a.db.warehouseStock.iter()).filter(s=>s.gameId===game);
  const buildings=()=>Array.from(a.db.building.iter()).filter(s=>s.gameId===game);
  const workers=()=>Array.from(a.db.worker.iter()).filter(s=>s.gameId===game);
  const producers=()=>Array.from(a.db.producer.iter()).filter(s=>s.gameId===game);
  const sums=()=>warehouses().reduce((n,s)=>[n[0]+s.wood,n[1]+s.stone,n[2]+s.iron],[0,0,0]);
  const balances=()=>[balance().wood,balance().stone,balance().iron];
  await until(()=>!!balance(),'Initial balance');
  assert.deepEqual(balances(),[0,0,0]);assert.equal(warehouses().length,0);assert.equal(buildings().length,0);
  for(let i=0;i<3;i++) assert.equal(await call(a,'build',[-121,45+i*10,'housing']),'Committed');
  assert.equal(await call(a,'build',[-1,45,'stockpile']),'Committed');
  await until(()=>warehouses().length===1&&balance().stone===20,'First warehouse gift replicated');
  assert.deepEqual(balances(),[20,20,20]); assert.deepEqual(sums(),[20,20,20]);
  const warehouse=buildings().find(b=>b.kind==='stockpile')!;
  assert.equal(await call(a,'joinGame',[game]),'Committed');
  assert.equal(warehouses().length,1,'Repeated join does not create stock');
  assert.equal(await call(a,'build',[20,35,'quarry']),'Committed');
  assert.equal(await call(a,'build',[20,55,'iron_mine']),'Committed');
  assert.equal(await call(a,'build',[-20,35,'saw_pit']),'Committed');
  await until(()=>workers().length===3 && producers().length===3,'Producer worker creation');
  const workersBefore=workers().map(w=>a.db.armyUnit.id.find(w.unitId)!);
  const soldier=Array.from(a.db.armyUnit.iter()).find(u=>u.gameId===game&&u.number===1)!;
  assert.equal(await call(a,'setTargets',[[soldier.id,workers()[0].unitId],30,10]),'Failed','Workers cannot receive movement orders');
  assert.equal(a.db.armyUnit.id.find(soldier.id)!.targetX,soldier.targetX,'Mixed command rolls back');
  assert.equal(await call(a,'setFormationTargets',[[workers()[0].unitId],20,10,30,10]),'Failed');
  await until(()=>workers().some(w=>w.cargo>0),'Produced material is picked up');
  assert.deepEqual(balances(),[20,20,20],'Goods in transit are not spendable');
  assert(workers().every(w=>w.cargo<=5));
  assert.equal(await call(a,'buildWall',[9,39,9,45]),'Committed','Obstacle placed along transport route');
  await until(()=>balance().wood>20&&balance().stone>16&&balance().iron>20,'All producers deliver into storage');
  assert.deepEqual(balances(),sums(),'Balances equal physically stored material');
  assert(workers().some(w=>{const u=a.db.armyUnit.id.find(w.unitId)!;const old=workersBefore.find(v=>v.id===u.id)!;return Math.hypot(u.x-old.x,u.z-old.z)>1;}),'Transporters walk');
  await until(()=>Array.from(observer.db.warehouseStock.iter()).some(s=>s.gameId===game&&s.iron>20),'Observers see warehouse deliveries');
  const oldStone=balance().stone;
  assert.equal(await call(a,'build',[31,15,'tower_stairwell']),'Committed');
  assert.equal(balance().stone,oldStone-5);assert.deepEqual(balances(),sums(),'Tower withdraws physical stone');
  const before=buildings().length;
  const amount=balance().stone;
  assert.equal(await call(a,'buildWall',[-99,81,99,81]),'Failed','Unaffordable line rolls back');
  assert.equal(buildings().length,before);assert.equal(balance().stone,amount);assert.deepEqual(balances(),sums());
  assert.equal(await call(a,'removeBuilding',[warehouse.id]),'Committed');
  assert.deepEqual(balances(),[0,0,0],'Storage demolition removes all spendable goods');
  await delay(4000);
  assert.deepEqual(balances(),[0,0,0],'Production without a warehouse never credits balances');
  assert(producers().every(p=>p.buffer<=20),'Production buffers remain bounded');
  assert.equal(await call(a,'build',[-1,65,'stockpile']),'Committed');
  await until(()=>balance().wood>0&&balance().stone>0&&balance().iron>0,'Workers resume delivery to replacement storage');
  assert.deepEqual(balances(),sums());
  for(const producer of producers()) assert.equal(await call(a,'removeBuilding',[producer.buildingId]),'Committed');
  await until(()=>workers().length===0&&producers().length===0,'Demolition removes workers');
  for(const stock of warehouses()) assert.equal(await call(a,'removeBuilding',[stock.buildingId]),'Committed');
  assert.deepEqual(balances(),[0,0,0],'Demolished storage loses its stock');
  assert.equal(await call(a,'build',[warehouse.x,warehouse.z,'stockpile']),'Committed');
  assert.deepEqual(balances(),[0,0,0],'New warehouse is empty');
  assert.equal(await call(a,'leaveGame'),'Committed');
  assert.equal(await call(a,'joinGame',[game]),'Committed');
  await until(()=>!!balance()&&warehouses().length===0,'Rejoin starts without storage');
  assert.deepEqual(balances(),[0,0,0]);
  for(let i=0;i<3;i++) assert.equal(await call(a,'build',[-121,45+i*10,'housing']),'Committed');
  assert.equal(await call(a,'build',[-1,45,'stockpile']),'Committed');await until(()=>warehouses().length===1&&balance().stone===20,'New participation first warehouse gift');
  assert.deepEqual(balances(),[20,20,20]);assert.deepEqual(sums(),[20,20,20]);
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Browser bridge ready');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Logistics bridge ${Date.now()}`}));
  await until(()=>snapshot().game_id!==null&&snapshot().resources?.stone===0,'Browser starts empty');
  assert.equal(snapshot().warehouses.length,0);
  strongholdNetwork.send(JSON.stringify({action:'build',x:-121,z:45,building:'housing'}));
  await until(()=>snapshot().buildings.some((b:any)=>b.kind==='housing'),'Browser housing');
  strongholdNetwork.send(JSON.stringify({action:'build',x:-1,z:45,building:'stockpile'}));
  await until(()=>snapshot().resources?.stone===20&&snapshot().warehouses.length===1,'Browser receives first warehouse stock');
  strongholdNetwork.send(JSON.stringify({action:'build',x:20,z:35,building:'quarry'}));
  await until(()=>snapshot().workers.length===1,'Browser receives worker');
  const worker=snapshot().units.find((u:any)=>u.worker);
  assert(worker&&!worker.owned,'Workers are excluded from player selection');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null&&snapshot().warehouses.length===0,'Browser clears logistics');
  assert.equal(await call(a,'leaveGame'),'Committed');
  await until(()=>warehouses().length===0&&workers().length===0&&producers().length===0,'Participation cleanup');
  console.log('PASS: empty starts and first warehouse physical gifts, three producers, autonomous unselectable workers, real transport and delivery, costs, atomic rollback, replication, demolition, cleanup and browser snapshots.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
