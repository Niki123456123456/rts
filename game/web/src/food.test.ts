// First-granary gifts and physically stored food against a disposable server.
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
        .subscribe(['SELECT * FROM game','SELECT * FROM participant','SELECT * FROM army_unit','SELECT * FROM building','SELECT * FROM building_rotation','SELECT * FROM tower_occupant','SELECT * FROM player_resources','SELECT * FROM granary_stock']);
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
  assert.equal(await call(a,'createGame',[`Food ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  assert.equal(await call(b,'joinGame',[game]),'Committed');
  const stocks=(db=a)=>Array.from(db.db.granaryStock.iter()).filter(s=>s.gameId===game&&s.owner.toHexString()===db.connectionId.toHexString());
  const food=(db=a)=>stocks(db).reduce((n,s)=>[n[0]+s.apples,n[1]+s.bread,n[2]+s.cheese,n[3]+s.meat],[0,0,0,0]);
  assert.deepEqual(food(),[0,0,0,0]);assert.deepEqual(food(b),[0,0,0,0]);
  assert.equal(await call(a,'build',[999,999,'granary']),'Failed');
  assert.equal(stocks().length,0,'Invalid granary never receives food');
  assert.equal(await call(a,'buildRotated',[-1,45,'granary',1]),'Committed','No resources or peasants required');
  await until(()=>stocks().length===1,'First granary replicated');
  assert.deepEqual(food(),[5,5,5,5]);
  const first=stocks()[0];
  assert.equal(a.db.playerResources.connectionId.find(a.connectionId)!.wood,0,'Granary gift is independent of construction supplies');
  await until(()=>Array.from(b.db.granaryStock.iter()).some(s=>s.buildingId===first.buildingId&&s.apples===5),'Observers see physical food');
  assert.deepEqual(food(b),[0,0,0,0],'Other player has no food');
  assert.equal(await call(a,'build',[21,45,'granary']),'Committed');
  assert.deepEqual(food(),[5,5,5,5],'Additional granary is empty');
  assert.equal(await call(a,'joinGame',[game]),'Committed');
  assert.deepEqual(food(),[5,5,5,5],'Repeated join never replenishes food');
  assert.equal(await call(b,'build',[41,45,'granary']),'Committed');
  await until(()=>stocks(b).length===1,'Other player granary');
  assert.deepEqual(food(b),[5,5,5,5],'Each participant receives own first gift');
  assert.equal(await call(a,'removeBuilding',[first.buildingId]),'Committed');
  await until(()=>!a.db.granaryStock.buildingId.find(first.buildingId),'Demolished granary removes stored food');
  assert.deepEqual(food(),[0,0,0,0]);
  for(const stock of stocks()) assert.equal(await call(a,'removeBuilding',[stock.buildingId]),'Committed');
  assert.equal(await call(a,'build',[-1,45,'granary']),'Committed');
  assert.deepEqual(food(),[0,0,0,0],'Rebuilding after all granaries are removed never repeats gift');
  assert.equal(await call(a,'leaveGame'),'Committed');
  await until(()=>stocks().length===0,'Leaving removes physical food rows');
  assert.equal(await call(a,'joinGame',[game]),'Committed');
  assert.equal(await call(a,'build',[-1,45,'granary']),'Committed');
  await until(()=>stocks().length===1,'New participation granary');
  assert.deepEqual(food(),[5,5,5,5]);
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Browser ready');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Food bridge ${Date.now()}`}));
  await until(()=>snapshot().game_id!==null&&snapshot().resources?.apples===0,'Browser starts with zero food');
  assert.deepEqual(snapshot().granaries,[]);
  strongholdNetwork.send(JSON.stringify({action:'build',x:-1,z:45,building:'granary'}));
  await until(()=>snapshot().resources?.apples===5&&snapshot().granaries.length===1,'Browser receives first granary food');
  assert.deepEqual(snapshot().resources,{wood:0,stone:0,iron:0,apples:5,bread:5,cheese:5,meat:5});
  const id=snapshot().granaries[0].building_id;
  strongholdNetwork.send(JSON.stringify({action:'remove_building',building_id:id}));
  await until(()=>snapshot().granaries.length===0&&snapshot().resources?.meat===0,'Browser removes destroyed stock and food totals');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null&&snapshot().granaries.length===0&&snapshot().resources===null,'Browser clears food on leave');
  assert.equal(await call(a,'leaveGame'),'Committed');assert.equal(await call(b,'leaveGame'),'Committed');
  console.log('PASS: empty food start, per-player first granary gifts, exact physical stock, observers, no repeated gift, demolition, fresh participation and browser food totals/cleanup.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
