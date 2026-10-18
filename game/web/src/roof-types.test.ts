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
const profiles=[
  {kind:'tower_stairwell',height:6.663,spacing:0.7,size:5,hatch:[-0.75,0.9],doors:[[-2.7,1.04],[-0.99,2.7]]},
  {kind:'tower_lookout',height:15.513,spacing:0.7,size:5,hatch:[-0.782,0.85],doors:[[-2.9,0.968],[-1.055,2.9]]},
  {kind:'tower_square',height:14.399,spacing:0.7,size:8,hatch:[0.274,1.796],doors:[[0.02,4.1]]},
];
function rotate(x:number,z:number,q:number):[number,number] {
  return q===0?[x,z]:q===1?[z,-x]:q===2?[-x,-z]:[-z,x];
}
try {
  const a=await connect(),b=await connect();
  const call=(db:DbConnection,name:string,args:unknown[])=>status(db,cb=>(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)](cb),()=>(db.reducers as any)[name](...args));
  assert.equal(await call(a,'createGame',[`Roof models ${Date.now()}`]),'Committed');
  const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
  assert.equal(await call(b,'joinGame',[game]),'Committed');
  const own=()=>Array.from(a.db.armyUnit.iter()).filter(u=>u.gameId===game && u.owner.toHexString()===a.connectionId.toHexString());
  await until(()=>own().length===20,'Army replicated');
  const ids=own().map(u=>u.id);
  const buildings=()=>Array.from(a.db.building.iter()).filter(v=>v.gameId===game);
  let target:{row:ReturnType<typeof buildings>[number];profile:typeof profiles[number];q:number}|undefined;
  let arrivals=0;
  a.db.armyUnit.onUpdate((_ctx,old,u)=>{
    if(!ids.includes(u.id))return;
    assert.ok([0.75,...profiles.map(p=>p.height+0.75)].some(y=>Math.abs(y-u.y)<0.002),'Every unit stands on ground or a model roof, never in the stairs');
    if(old.y===0.75 && u.y>1 && target) {
      const {row,profile,q}=target;
      const hatch=rotate(profile.hatch[0],profile.hatch[1],q);
      const nearDoor=profile.doors.some(([x,z])=>{const d=rotate(x,z,q);return Math.hypot(old.x-row.x-d[0],old.z-row.z-d[1])<=0.551;});
      assert.ok(nearDoor,`${profile.kind}: starts at a correctly rotated doorway`);
      assert.ok(Math.hypot(u.x-row.x-hatch[0],u.z-row.z-hatch[1])<=0.151,`${profile.kind}: exits at its own hatch`);
      assert.ok(Math.abs(u.y-row.y-profile.height-0.75)<0.002);
      arrivals++;
    }
  });
  for(const [index,profile] of profiles.entries()) {
    for(let q=0;q<4;q++) {
      const x=10+q*12,z=18+index*16;
      const count=buildings().length;
      assert.equal(await call(a,'buildRotated',[x,z,profile.kind,q]),'Committed');
      await until(()=>buildings().length===count+1,'Model replicated');
      const row=buildings().find(v=>v.kind===profile.kind && Math.abs(v.x-x)<=1.01 && Math.abs(v.z-z)<=1.01)!;
      target={row,profile,q};
      const selected=q===0?ids:ids.slice(0,4);
      assert.equal(await call(a,'moveToRoof',[selected,row.id,row.x,row.z]),'Committed',`${profile.kind} orientation ${q}`);
      await until(()=>selected.every(id=>{
        const u=a.db.armyUnit.id.find(id)!;
        return a.db.towerOccupant.unitId.find(id)?.buildingId===row.id && Math.abs(u.y-row.y-profile.height-0.75)<0.002 && u.x===u.targetX && u.z===u.targetZ;
      }),`Units reach ${profile.kind} roof at rotation ${q}`,45000);
      for(const id of selected) {
        const u=a.db.armyUnit.id.find(id)!;
        const local=rotate(u.x-row.x,u.z-row.z,(4-q)%4);
        for(const coordinate of local) assert.ok(Array.from({length:profile.size},(_,i)=>i-(profile.size-1)/2).some(v=>Math.abs(v*profile.spacing-coordinate)<0.002),'Roof grid follows usable area at a constant pitch');
      }
      if(profile.kind==='tower_square') assert.ok(selected.some(id=>a.db.towerOccupant.unitId.find(id)!.slot>=25),'Server allocates slots beyond the former capacity');
      assert.equal(new Set(selected.map(id=>a.db.towerOccupant.unitId.find(id)!.slot)).size,selected.length);
      await until(()=>selected.every(id=>Math.abs(b.db.armyUnit.id.find(id)!.y-row.y-profile.height-0.75)<0.002),'Roof height replicates to second player');
      assert.equal(await call(b,'moveToRoof',[[Array.from(b.db.armyUnit.iter()).find(u=>u.owner.toHexString()===b.connectionId.toHexString())!.id],row.id,row.x,row.z]),'Failed','Enemy cannot use the roof');
    }
    console.log(`PASS: ${profile.kind}, model roof height, ${profile.size*profile.size} slots at 0.7 m, doors/hatch, all four rotations and multiplayer.`);
  }
  assert.ok(arrivals>=84,'All models and rotations performed real door-to-hatch transfers');
  // The square tower has one door. Blocking that door must reject new ground entrants.
  assert.equal(await call(a,'setTargets',[[ids[0]],60,65]),'Committed');
  await until(()=>!a.db.towerOccupant.unitId.find(ids[0]),'Ground order descends from the square tower');
  const square=buildings().find(v=>v.kind==='tower_square' && v.x===11)!;
  assert.equal(await call(a,'buildRotated',[12,56,'tower_stairwell',0]),'Committed');
  await until(()=>buildings().some(v=>v.kind==='tower_stairwell' && v.x===12 && v.z===56),'Door blocker replicated');
  assert.equal(await call(a,'moveToRoof',[[ids[0]],square.id,square.x,square.z]),'Failed','Single blocked door rejects approach');
  assert.equal(await call(a,'build',[90,90,'granary']),'Committed');
  await until(()=>buildings().some(v=>v.kind==='granary'),'Unsupported roof replicated');
  const granary=buildings().find(v=>v.kind==='granary')!;
  assert.equal(await call(a,'moveToRoof',[[ids[0]],granary.id,granary.x,granary.z]),'Failed','Unsupported buildings stay non-walkable');
  const last=buildings().find(v=>v.kind==='tower_square' && v.x===47)!;
  assert.equal(await call(a,'removeBuilding',[last.id]),'Committed');
  await until(()=>ids.slice(1,4).every(id=>!a.db.towerOccupant.unitId.find(id)),'Demolition evacuates square roof');
  assert.ok(ids.slice(1,4).every(id=>a.db.armyUnit.id.find(id)!.y===0.75));
  target=undefined;
  // The browser bridge uses the same generic action for both additional models.
  strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));
  const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
  await until(()=>snapshot().ready,'Bridge connected');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Generic roofs bridge ${Date.now()}`}));
  await until(()=>snapshot().units.length===20,'Bridge army replicated');
  for(const kind of ['tower_lookout','tower_square']) {
    const x=kind==='tower_lookout'?10:22;
    strongholdNetwork.send(JSON.stringify({action:'build',building:kind,x,z:18,rotation:1}));
    await until(()=>snapshot().buildings.some((v:any)=>v.kind===kind),'Bridge model replicated');
    const state=snapshot(),row=state.buildings.find((v:any)=>v.kind===kind);
    strongholdNetwork.send(JSON.stringify({action:'move_to_roof',unit_ids:state.units.map((u:any)=>u.id),building_id:row.id,x:row.x,z:row.z}));
    const height=profiles.find(p=>p.kind===kind)!.height+0.75;
    await until(()=>snapshot().units.every((u:any)=>Math.abs(u.y-height)<0.002),'Bridge displays the model roof height');
  }
  strongholdNetwork.send(JSON.stringify({action:'leave'}));
  await until(()=>snapshot().game_id===null,'Bridge cleanup');
  assert.equal(await call(a,'leaveGame',[]),'Committed');
  await until(()=>!Array.from(a.db.towerOccupant.iter()).some(o=>o.gameId===game),'All roof types clean up on leave');
  console.log('PASS: generic roof transfers, single-door obstruction, unsupported buildings, demolition and both browser models.');
} finally {strongholdNetwork.disconnect();for(const client of clients)client.disconnect();}
