// Real-server needs/workforce test in an isolated database.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
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
        'SELECT * FROM keep_population','SELECT * FROM settlement_needs','SELECT * FROM building_operation']);})
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
  assert.ok(startingScore>=80,'new settlements start happy without advanced services');
  assert.equal(needs().immigrationSeconds,4);
  assert.equal(needs().water,0);
  assert.equal(needs().variety,0);
  assert.equal((await build('apple_farm',0,24)).tag,'Committed','empty workplaces can be constructed before peasants arrive');
  const farm=ownBuildings().find(row=>row.kind==='apple_farm')!;
  const workers=()=>Array.from(a.db.worker.iter()).filter(w=>w.sourceBuilding===farm.id);
  const open=(db=a)=>status(db,cb=>db.reducers.onAssignPeasant(cb),()=>db.reducers.assignPeasant(farm.id));
  const close=(db=a)=>status(db,cb=>db.reducers.onUnassignPeasant(cb),()=>db.reducers.unassignPeasant(farm.id));
  assert.equal((await close()).tag,'Committed','an empty workplace can be closed');
  assert.equal((await build('housing',-17,17)).tag,'Committed','obstacle between camp and farm');
  assert.equal((await build('granary',-9,-23)).tag,'Committed');
  assert.equal((await build('well',13,-19)).tag,'Committed');
  const granary=ownBuildings().find(row=>row.kind==='granary')!;
  const well=ownBuildings().find(row=>row.kind==='well')!;
  const initialBread=a.db.granaryStock.buildingId.find(granary.id)!.bread;
  await until(()=>population().arrived>=1,'first settler arrives',10000);
  assert.equal(workers().length,0,'closed buildings stay unstaffed when peasants arrive');
  assert.equal(a.db.producer.buildingId.find(farm.id)!.buffer,0);
  assert.equal((await open()).tag,'Committed','opening automatically assigns an available peasant');
  await until(()=>workers().length===1,'automatic assignment creates a worker');
  const unit=a.db.armyUnit.id.find(workers()[0].unitId)!;
  assert.ok(Math.hypot(unit.x-keep.x,unit.z-(keep.z+11))<6,'worker starts at the campfire');
  assert.equal((await open()).tag,'Committed','opening is idempotent');
  assert.equal(workers().length,1,'no duplicate assignment');
  const bounds=new Map<string,[number,number]>();
  const catalog=readFileSync(new URL('../../../shared/src/building_catalog.rs',import.meta.url),'utf8');
  for(const match of catalog.matchAll(/name: "([^"]+)",[\s\S]*?min: \[([^\]]+)\],[\s\S]*?max: \[([^\]]+)\]/g)) {
    const min=match[2].split(',').map(Number), max=match[3].split(',').map(Number);
    bounds.set(match[1],[(max[0]-min[0])*.005,(max[2]-min[2])*.005]);
  }
  assert.ok(bounds.has('housing'));
  let collision:string|undefined;
  let samples=0;
  const monitor=setInterval(()=>{
    for(const worker of workers()) {
      const u=a.db.armyUnit.id.find(worker.unitId); if(!u)continue; samples++;
      for(const b of ownBuildings()) {
        const size=bounds.get(b.kind); if(!size)continue;
        const half=size; // All buildings in this test are placed at rotation 0.
        if(Math.abs(u.x-b.x)<half[0]+.24 && Math.abs(u.z-b.z)<half[1]+.24)
          collision=`worker crossed ${b.kind} at ${u.x},${u.z}`;
      }
    }
  },20);
  try {
    await until(()=>a.db.producer.buildingId.find(farm.id)!.buffer>0 || workers()[0]?.cargo>0,'worker walks around the house and starts producing',30000);
  } finally {clearInterval(monitor);}
  assert.ok(samples>10,'observed worker movement');
  assert.equal(collision,undefined);
  assert.equal((await status(b,cb=>b.reducers.onJoinGame(cb),()=>b.reducers.joinGame(game))).tag,'Committed');
  assert.equal((await close(b)).tag,'Failed','other players cannot close your workplace');
  assert.equal((await open(b)).tag,'Failed','other players cannot open your workplace');
  assert.equal((await close()).tag,'Committed');
  await until(()=>workers().length===0,'closing frees its worker');
  const buffer=a.db.producer.buildingId.find(farm.id)!.buffer;
  await delay(4500);
  assert.equal(a.db.producer.buildingId.find(farm.id)!.buffer,buffer,'closed buildings stop producing');
  assert.equal(workers().length,0,'automatic assignment never reopens a closed building');
  await until(()=>population().arrived>=8 && needs().water>0,'water becomes relevant at eight settlers',45000);
  assert.equal(needs().variety,0);
  assert.ok(needs().happiness>startingScore);
  await until(()=>b.db.buildingOperation.buildingId.find(farm.id)?.closed===true,'closed state replicates');
  for(const [kind,x,z] of [['saw_pit',40,-43],['quarry',-65,-35],['iron_mine',-80,15]] as const) {
    assert.equal((await build(kind,x,z)).tag,'Committed',`${kind} can be built on a nearby resource`);
    const building=ownBuildings().find(row=>row.kind===kind)!;
    await until(()=>Array.from(a.db.worker.iter()).some(w=>w.sourceBuilding===building.id),`${kind} automatically receives a free peasant`);
  }
  assert.equal((await open()).tag,'Committed');
  await until(()=>workers().length===1,'reopening automatically restaffs the farm');
  assert.equal((await close()).tag,'Committed');
  await until(()=>a.db.granaryStock.buildingId.find(granary.id)!.bread<initialBread,'population consumes actual granary rations',35000);
  assert.equal((await remove(granary.id)).tag,'Committed');
  assert.equal((await remove(well.id)).tag,'Committed');
  await until(()=>needs().food<0 && needs().water<0 && needs().immigrationSeconds===0,'food and water matter once the village grows');
  assert.ok(needs().happiness<50);
  console.log('PASS: easy start, progressive needs, water at eight settlers, fast immigration, real food consumption, automatic staffing, closed workplace persistence, campfire departure, building collision, nearby resources, ownership, duplicate protection, production gating, reopening, replication and shortages after village growth.');
}finally{for(const client of clients)client.disconnect();}
