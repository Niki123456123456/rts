// Real-server checks for balances and atomic physical stock withdrawals.
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
  const a=await connect(),b=await connect();
  const call=(db:DbConnection,name:string,args:unknown[])=>status(db,cb=>(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)](cb),()=>(db.reducers as any)[name](...args));
  assert.equal(await call(a,'createGame',[`Resources ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  assert.equal(await call(b,'joinGame',[game]),'Committed');
  const stock=(db:DbConnection)=>db.db.playerResources.connectionId.find(db.connectionId)!;
  await until(()=>!!stock(a)&&!!stock(b),'Starting balances replicated');
  const balances=(db:DbConnection)=>{const r=stock(db);return [r.wood,r.stone,r.iron];};
  assert.deepEqual(balances(a),[20,20,20]);assert.deepEqual(balances(b),[20,20,20]);
  const buildings=()=>Array.from(a.db.building.iter()).filter(v=>v.gameId===game);
  const own=()=>buildings().filter(v=>v.owner.toHexString()===a.connectionId.toHexString());
  assert.equal(await call(a,'buildRotated',[10,18,'tower_stairwell',1]),'Committed');
  await until(()=>stock(a).stone===15,'Tower costs five stone');
  assert.deepEqual(balances(b),[20,20,20],'Other player balance is independent');
  assert.equal(await call(a,'build',[10,18,'tower_stairwell']),'Failed');
  assert.equal(stock(a).stone,15,'Invalid construction has no cost');
  assert.equal(await call(a,'buildWall',[13,19,17,19]),'Committed');
  await until(()=>stock(a).stone===12&&own().length===5,'Three wall cells cost three stone');
  assert.equal(await call(a,'buildWall',[17,19,21,19]),'Committed');
  await until(()=>stock(a).stone===10&&own().length===7,'Extending a wall only charges new cells');
  assert.equal(await call(a,'buildWall',[21,19,13,19]),'Committed');
  assert.equal(stock(a).stone,10,'Existing segments are free to reuse');
  assert.equal(await call(a,'buildWall',[23,19,9,19]),'Failed');
  assert.equal(stock(a).stone,10);assert.equal(own().length,7,'Invalid line rolls back its first segment and payment');
  assert.equal(await call(a,'buildWall',[-99,61,99,61]),'Failed','Insufficient stone rejects whole line');
  assert.equal(stock(a).stone,10);assert.equal(own().length,7,'Insufficient balance rolls back partial construction and withdrawals');
  assert.equal(await call(a,'buildWall',[41,81,53,81]),'Committed');
  await until(()=>stock(a).stone===3,'Seven segments charge exactly seven stone');
  assert.equal(await call(a,'buildRotated',[31,25,'tower_square',3]),'Failed','Cannot afford tower');
  assert.equal(stock(a).stone,3);
  assert.equal(await call(a,'buildWall',[41,85,45,85]),'Committed');
  await until(()=>stock(a).stone===0,'Balance can reach zero without underflow');
  assert.equal(await call(a,'buildWall',[41,85,45,85]),'Committed','Zero balance still allows reuse');
  assert.equal(await call(a,'joinGame',[game]),'Committed');
  assert.equal(stock(a).stone,0,'Repeated join never resets balance');
  assert.equal(await call(a,'build',[31,35,'granary']),'Committed','Other building types remain free');
  assert.deepEqual(balances(a),[20,0,20],'Wood and iron are preserved');
  const granary=own().find(v=>v.kind==='granary')!;
  assert.equal(await call(a,'removeBuilding',[granary.id]),'Committed');
  assert.equal(stock(a).stone,0,'Demolition does not mint resources');
  assert.equal(await call(b,'build',[31,25,'tower_wooden']),'Committed');
  await until(()=>stock(b).stone===15,'All tower kinds use the shared cost');
  assert.equal(stock(a).stone,0);
  assert.equal(await call(a,'leaveGame',[]),'Committed');
  await until(()=>!stock(a),'Leave removes resource row');
  assert.equal(await call(a,'joinGame',[game]),'Committed');
  await until(()=>!!stock(a),'New participation receives starting stock');
  assert.deepEqual(balances(a),[20,20,20]);
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Browser bridge ready');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Resource bridge ${Date.now()}`}));
  await until(()=>snapshot().resources?.stone===20,'Browser snapshot receives starting resources');
  assert.deepEqual(snapshot().resources,{wood:20,stone:20,iron:20});
  strongholdNetwork.send(JSON.stringify({action:'build_wall',start_x:13,start_z:19,end_x:17,end_z:19}));
  await until(()=>snapshot().resources?.stone===17,'Browser snapshot receives wall payment');
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null&&snapshot().resources===null,'Browser clears resource display on leave');
  assert.equal(await call(a,'leaveGame',[]),'Committed');
  assert.equal(await call(b,'leaveGame',[]),'Committed');
  await until(()=>!Array.from(a.db.playerResources.iter()).some(r=>r.gameId===game),'Game resource rows clean up');
  console.log('PASS: 20 starting wood/stone/iron, per-player balances, tower and wall costs, atomic rollback, affordability, reuse, join/leave and browser snapshots.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
