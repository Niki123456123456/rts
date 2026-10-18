import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri=process.env.SPACETIMEDB_URI??'http://127.0.0.1:3300', database=process.env.SPACETIMEDB_DATABASE??'stronghold-combat';
const clients:DbConnection[]=[];
const sleep=(ms:number)=>new Promise(r=>setTimeout(r,ms));
async function until(f:()=>boolean,label:string,ms=45000){const end=Date.now()+ms;while(!f()){assert(Date.now()<end,label);await sleep(25)}}
async function connect(){return new Promise<DbConnection>((resolve,reject)=>DbConnection.builder().withUri(uri).withModuleName(database).onConnect(db=>{clients.push(db);db.subscriptionBuilder().onApplied(()=>resolve(db)).onError(c=>reject(c.event)).subscribe(['game','participant','army_unit','building','building_rotation','unit_profile','player_resources','worker'].map(t=>`SELECT * FROM ${t}`));}).onConnectError((_,e)=>reject(e)).build());}
function call(db:DbConnection,name:string,args:unknown[]=[]){return new Promise<string>((resolve,reject)=>{let done=false;const timer=setTimeout(()=>reject(new Error(`Timeout ${name}`)),8000);(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)]((c:ReducerEventContext)=>{if(done||c.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;done=true;clearTimeout(timer);resolve(c.event.status.tag)});(db.reducers as any)[name](...args)});}
try{
 const a=await connect(),b=await connect();
 assert.equal(await call(a,'createGame',[`Combat ${Date.now()}`]),'Committed');
 const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;
 const own=()=>Array.from(a.db.armyUnit.iter()).filter(u=>u.owner.toHexString()===a.connectionId.toHexString());
 const buildings=()=>Array.from(a.db.building.iter()).filter(v=>v.gameId===game);
 await until(()=>own().length===1,'One starting unit');
 const lord=own()[0];assert.equal(a.db.unitProfile.unitId.find(lord.id)!.kind,'lord');
 assert.equal(await call(a,'recruitUnit',[999999n,'armed_peasant']),'Failed');
 assert.equal(await call(b,'joinGame',[game]),'Committed');
 assert.equal(await call(a,'build',[10,18,'barracks']),'Committed');
 const barracks=buildings().find(v=>v.kind==='barracks')!;
 assert.equal(await call(b,'recruitUnit',[barracks.id,'armed_peasant']),'Failed','Foreign barracks rejected');
 assert.equal(await call(a,'recruitUnit',[barracks.id,'lord']),'Failed','Cannot recruit additional lords');
 for(let i=0;i<3;i++)assert.equal(await call(a,'recruitUnit',[barracks.id,'armed_peasant']),'Committed');
 assert.equal(own().length,4);
 const peasants=own().filter(u=>u.id!==lord.id);
 for(const p of peasants){const profile=a.db.unitProfile.unitId.find(p.id)!;assert.equal(profile.kind,'armed_peasant');assert.equal(profile.activity,0);assert(p.z>barracks.z,'Units spawn in front of barracks');}
 for(let i=0;i<peasants.length;i++)for(let j=i+1;j<peasants.length;j++)assert(Math.hypot(peasants[i].x-peasants[j].x,peasants[i].z-peasants[j].z)>=.699);
 const p=peasants[0];
 assert.equal(await call(a,'setTargets',[[p.id],30,10]),'Committed');
 await until(()=>a.db.unitProfile.unitId.find(p.id)?.activity===1,'Walking state');
 await until(()=>a.db.unitProfile.unitId.find(p.id)?.activity===0&&Math.hypot(a.db.armyUnit.id.find(p.id)!.x-30,a.db.armyUnit.id.find(p.id)!.z-10)<.1,'Idle on arrival');
 assert.equal(await call(b,'build',[40,10,'granary']),'Committed');
 await until(()=>buildings().some(v=>v.kind==='granary'),'Enemy building replication');
 const enemy=buildings().find(v=>v.kind==='granary')!;
 const hp=enemy.health;
 assert.equal(await call(a,'attackBuilding',[[p.id],enemy.id]),'Committed');
 await until(()=>a.db.unitProfile.unitId.find(p.id)?.activity===2,'Fighting begins in attack range');
 await until(()=>a.db.building.id.find(enemy.id)!.health<hp,'Fighting causes server damage');
 await until(()=>b.db.unitProfile.unitId.find(p.id)?.activity===2,'Observer receives fighting state');
 assert.equal(await call(a,'setTargets',[[p.id],30,10]),'Committed');
 await until(()=>a.db.unitProfile.unitId.find(p.id)?.activity===1,'Combat switches back to walking');
 assert.equal(await call(a,'joinGame',[game]),'Committed');assert.equal(own().length,4,'Repeated join preserves army');
 strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());await until(()=>snapshot().ready,'Browser ready');
 strongholdNetwork.send(JSON.stringify({action:'create',title:`Combat bridge ${Date.now()}`}));await until(()=>snapshot().units.length===1,'Browser has one lord');assert.equal(snapshot().units[0].kind,'lord');
 strongholdNetwork.send(JSON.stringify({action:'build',building:'barracks',x:10,z:18}));await until(()=>snapshot().buildings.some((b:any)=>b.kind==='barracks'),'Browser barracks');
 strongholdNetwork.send(JSON.stringify({action:'recruit_unit',building_id:snapshot().buildings.find((b:any)=>b.kind==='barracks').id,kind:'armed_peasant'}));await until(()=>snapshot().units.some((u:any)=>u.kind==='armed_peasant'),'Browser recruitment and typed snapshot');
 strongholdNetwork.send(JSON.stringify({action:'leave'}));await until(()=>snapshot().game_id===null,'Browser leaves');
 assert.equal(await call(a,'leaveGame'),'Committed');assert.equal(await call(b,'leaveGame'),'Committed');await until(()=>!Array.from(a.db.unitProfile.iter()).some(p=>p.gameId===game),'Profiles cleaned up');
 console.log('PASS: one starting lord, typed recruitment, barracks ownership, front spawning, spacing, idle/walking/fighting transitions, damage, replication, browser commands and cleanup.');
}finally{strongholdNetwork.disconnect();for(const c of clients)c.disconnect()}
