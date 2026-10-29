// Real-server needs/workforce test in an isolated database.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
const clients:DbConnection[]=[];
const delay=(ms:number)=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(test:()=>boolean,label:string,timeout=15000){
  const deadline=Date.now()+timeout;
  while(!test()){assert.ok(Date.now()<deadline,label);await delay(40);}
}
async function connect(){return new Promise<DbConnection>((resolve,reject)=>{
  DbConnection.builder().withUri('http://127.0.0.1:18766').withModuleName('stronghold-terrain-test')
    .onConnect(db=>{clients.push(db);db.subscriptionBuilder().onApplied(()=>resolve(db)).onError(ctx=>reject(ctx.event))
      .subscribe(['SELECT * FROM participant','SELECT * FROM building','SELECT * FROM army_unit',
        'SELECT * FROM worker','SELECT * FROM producer','SELECT * FROM granary_stock',
        'SELECT * FROM keep_population','SELECT * FROM settlement_needs']);})
    .onConnectError((_ctx,error)=>reject(error)).build();
});}
function status(db:DbConnection,register:(cb:(ctx:ReducerEventContext)=>void)=>void,call:()=>void){
  return new Promise<ReducerEventContext['event']['status']>((resolve,reject)=>{
    let done=false;const timer=setTimeout(()=>reject(new Error('Reducer timeout')),10000);
    register(ctx=>{if(done||ctx.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;
      done=true;clearTimeout(timer);resolve(ctx.event.status);});call();
  });
}
try{
  const a=await connect(), b=await connect();
  assert.equal((await status(a,cb=>a.reducers.onCreateGame(cb),()=>a.reducers.createGame(`Needs test ${Date.now()}`))).tag,'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  const ownBuildings=()=>Array.from(a.db.building.iter()).filter(row=>row.owner.toHexString()===a.connectionId.toHexString()&&row.gameId===game);
  const keep=ownBuildings().find(row=>row.kind==='keep')!;
  const needs=()=>a.db.settlementNeeds.buildingId.find(keep.id)!;
  const population=()=>a.db.keepPopulation.buildingId.find(keep.id)!;
  const build=(kind:string,x:number,z:number)=>status(a,cb=>a.reducers.onBuildRotated(cb),()=>a.reducers.buildRotated(x,z,kind,0));
  const remove=(id:bigint)=>status(a,cb=>a.reducers.onRemoveBuilding(cb),()=>a.reducers.removeBuilding(id));
  await until(()=>!!needs(),'initial satisfaction published');
  const startingScore=needs().happiness;
  assert.equal(needs().immigrationSeconds,14);
  assert.equal((await build('apple_farm',0,24)).tag,'Committed','empty workplaces can be constructed before peasants arrive');
  const farm=ownBuildings().find(row=>row.kind==='apple_farm')!;
  const workers=()=>Array.from(a.db.worker.iter()).filter(w=>w.sourceBuilding===farm.id);
  const assign=(db=a)=>status(db,cb=>db.reducers.onAssignPeasant(cb),()=>db.reducers.assignPeasant(farm.id));
  assert.equal((await assign()).tag,'Failed','assignment needs an actual free peasant');
  await delay(2200);
  assert.equal(a.db.producer.buildingId.find(farm.id)!.buffer,0,'unassigned buildings cannot produce');
  assert.equal(workers().length,0);
  assert.equal((await build('granary',-9,-23)).tag,'Committed');
  assert.equal((await build('well',13,-19)).tag,'Committed');
  assert.equal((await build('housing',-5,43)).tag,'Committed');
  const granary=ownBuildings().find(row=>row.kind==='granary')!;
  const initialBread=a.db.granaryStock.buildingId.find(granary.id)!.bread;
  const well=ownBuildings().find(row=>row.kind==='well')!;
  await until(()=>needs().happiness>=80 && needs().water>0 && needs().variety>0,'food diversity and water raise satisfaction');
  assert.ok(needs().happiness>startingScore);
  assert.equal(needs().immigrationSeconds,4,'good supply accelerates immigration');
  await until(()=>population().arrived>=1,'satisfied peasants arrive');
  assert.equal((await status(b,cb=>b.reducers.onJoinGame(cb),()=>b.reducers.joinGame(game))).tag,'Committed');
  assert.equal((await assign(b)).tag,'Failed','other players cannot staff your buildings');
  assert.equal((await assign()).tag,'Committed');
  assert.equal((await assign()).tag,'Failed','one worker per production building');
  await until(()=>workers().length===1,'assignment creates a production worker');
  const unit=a.db.armyUnit.id.find(workers()[0].unitId)!;
  assert.ok(Math.hypot(unit.x-keep.x,unit.z-(keep.z+11))<5,'assigned peasant starts at the campfire');
  await until(()=>a.db.producer.buildingId.find(farm.id)!.buffer>0 || workers()[0]?.cargo>0,'worker reaches workplace and production starts',30000);
  await until(()=>b.db.settlementNeeds.buildingId.find(keep.id)?.happiness===needs().happiness,'satisfaction replicates to other clients');
  assert.equal((await status(a,cb=>a.reducers.onUnassignPeasant(cb),()=>a.reducers.unassignPeasant(farm.id))).tag,'Committed');
  await until(()=>workers().length===0,'released worker returns to the free population');
  const buffer=a.db.producer.buildingId.find(farm.id)!.buffer;
  await delay(3500);
  assert.equal(a.db.producer.buildingId.find(farm.id)!.buffer,buffer,'production stops without its worker');
  await until(()=>a.db.granaryStock.buildingId.find(granary.id)!.bread<initialBread,'population consumes actual granary rations',35000);
  assert.equal((await remove(granary.id)).tag,'Committed');
  await until(()=>needs().food<0 && needs().immigrationSeconds===0,'food shortage stops immigration');
  assert.equal((await remove(well.id)).tag,'Committed');
  await until(()=>needs().happiness<40,'missing food and water cause low satisfaction');
  const before=population().arrived;
  assert.ok(before>1,'there are free peasants to emigrate');
  await until(()=>population().arrived<before,'dissatisfied free peasants emigrate',20000);
  console.log('PASS: combined needs, food variety, water, faster immigration, real food consumption, empty workplace, explicit staffing, campfire departure, ownership, duplicate protection, production gating, release, replication and emigration.');
}finally{for(const client of clients)client.disconnect();}
