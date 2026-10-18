import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri=process.env.SPACETIMEDB_URI??'http://127.0.0.1:3300', database=process.env.SPACETIMEDB_DATABASE??'stronghold-wall-combat';
const clients:DbConnection[]=[];
const sleep=(ms:number)=>new Promise(r=>setTimeout(r,ms));
async function until(f:()=>boolean,label:string,ms=45000){const end=Date.now()+ms;while(!f()){assert(Date.now()<end,label);await sleep(25)}}
async function connect(){return new Promise<DbConnection>((resolve,reject)=>DbConnection.builder().withUri(uri).withModuleName(database).onConnect(db=>{clients.push(db);db.subscriptionBuilder().onApplied(()=>resolve(db)).onError(c=>reject(c.event)).subscribe(['game','participant','army_unit','building','building_rotation','unit_profile','player_resources','worker'].map(t=>`SELECT * FROM ${t}`));}).onConnectError((_,e)=>reject(e)).build());}
function call(db:DbConnection,name:string,args:unknown[]=[]){return new Promise<string>((resolve,reject)=>{let done=false;const timer=setTimeout(()=>reject(new Error(`Timeout ${name}`)),8000);(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)]((c:ReducerEventContext)=>{if(done||c.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;done=true;clearTimeout(timer);resolve(c.event.status.tag)});(db.reducers as any)[name](...args)});}
try {
 const a=await connect(),b=await connect();
 const army=(db:DbConnection)=>Array.from(a.db.armyUnit.iter()).filter(u=>u.owner.toHexString()===db.connectionId.toHexString());
 const unit=(id:bigint)=>a.db.armyUnit.id.find(id);
 const activity=(id:bigint)=>a.db.unitProfile.unitId.find(id)?.activity;
 const ownBuildings=(db:DbConnection)=>Array.from(db.db.building.iter()).filter(v=>v.owner.toHexString()===db.connectionId.toHexString());
 async function fresh(label:string){await call(a,'leaveGame');await call(b,'leaveGame');assert.equal(await call(a,'createGame',[`${label} ${Date.now()}`]),'Committed');const game=Array.from(a.db.participant.iter()).find(p=>p.connectionId.toHexString()===a.connectionId.toHexString())!.gameId;assert.equal(await call(b,'joinGame',[game]),'Committed');await until(()=>army(a).length===1&&army(b).length===1,'Armies');}
 async function move(db:DbConnection,id:bigint,x:number,z:number){assert.equal(await call(db,'setTargets',[[id],x,z]),'Committed');}
 async function wall(kind='tower_stairwell'){
  assert.equal(await call(b,'build',[10,18,kind]),'Committed');
  assert.equal(await call(b,'buildWall',[13,19,17,19]),'Committed');
  return ownBuildings(b).find(v=>v.kind==='stone_wall'&&v.x===17)!;
 }
 await fresh('Wall counterattack');
 const remote=army(a)[0],defender=army(b)[0];await move(a,remote.id,-60,40);
 assert.equal(await call(a,'build',[-30,18,'barracks']),'Committed');
 const barracks=ownBuildings(a).find(v=>v.kind==='barracks')!;
 assert.equal(await call(a,'recruitUnit',[barracks.id,'armed_peasant']),'Committed');await until(()=>army(a).length===2,'Attacker recruits');
 const attacker=army(a).find(u=>u.number>1)!;await move(a,attacker.id,3,30);
 const target=await wall();assert.equal(await call(b,'moveToRoof',[[defender.id],target.id,17,19]),'Committed');
 await until(()=>unit(defender.id)!.y>7&&activity(defender.id)===0&&Math.hypot(unit(defender.id)!.x-17,unit(defender.id)!.z-19)<.1,'Defender on wall');
 await until(()=>Math.hypot(unit(attacker.id)!.x-3,unit(attacker.id)!.z-30)<.1,'Attacker stages');
 assert.equal(await call(a,'attackUnits',[[attacker.id],defender.id]),'Committed','Enemy wall is reachable via its tower');
 await until(()=>activity(attacker.id)===2&&activity(defender.id)===2,'Both fight on fortifications');
 assert(unit(attacker.id)!.y>7&&unit(defender.id)!.y>7,'No ground-to-roof damage');
 await until(()=>!unit(attacker.id),'Defender wins and releases attacker reservations');
 await until(()=>unit(defender.id)!.y>7&&Math.hypot(unit(defender.id)!.x-17,unit(defender.id)!.z-19)<.12&&activity(defender.id)===0,'Defender returns to original wall slot');
 assert(unit(defender.id)!.health>0&&unit(defender.id)!.health<1);
 console.log('PASS: ground-to-wall approach, enemy tower entry, reciprocal surface combat, casualty cleanup and roof return.');
 await fresh('Full wall');
 const hero=army(a)[0],otherLord=army(b)[0];await move(b,otherLord.id,60,-30);
 const full=await wall();
 assert.equal(await call(b,'build',[30,-20,'barracks']),'Committed');const bb=ownBuildings(b).find(v=>v.kind==='barracks')!;
 for(let i=0;i<9;i++)assert.equal(await call(b,'recruitUnit',[bb.id,'armed_peasant']),'Committed');await until(()=>army(b).length===10,'Nine defenders');
 const garrison=army(b).filter(u=>u.number>1);
 assert.equal(await call(b,'moveToRoof',[garrison.map(u=>u.id),full.id,17,19]),'Committed');
 await until(()=>garrison.every(u=>unit(u.id)!.y>7&&activity(u.id)===0),'Full garrison settles',60000);
 assert.equal(await call(a,'attackUnits',[[hero.id],garrison[0].id]),'Committed','Full target wall uses neighboring free surface slots');
 await until(()=>unit(hero.id)!.y>7&&garrison.some(u=>unit(u.id)!.health<1),'Full wall can be attacked',60000);
 assert(garrison.some(u=>activity(u.id)===2),'Full garrison retaliates');
 console.log('PASS: a full enemy wall can be attacked using its connected network.');
 await fresh('High tower entry and exit');
 const climber=army(a)[0],highLord=army(b)[0];await move(b,highLord.id,60,-30);
 assert.equal(await call(b,'build',[30,-20,'barracks']),'Committed');
 const highBarracks=ownBuildings(b).find(v=>v.kind==='barracks')!;
 assert.equal(await call(b,'recruitUnit',[highBarracks.id,'armed_peasant']),'Committed');await until(()=>army(b).length===2,'High tower defender');
 const highGuard=army(b).find(u=>u.number>1)!,highWall=await wall('tower_lookout');
 assert.equal(await call(b,'moveToRoof',[[highGuard.id],highWall.id,17,19]),'Committed');
 await until(()=>unit(highGuard.id)!.y>7&&unit(highGuard.id)!.y<8&&activity(highGuard.id)===0,'High tower garrison settles',60000);
 assert.equal(await call(a,'attackUnits',[[climber.id],highGuard.id]),'Committed');
 let climbedHigh=false;
 await until(()=>{climbedHigh ||= unit(climber.id)!.y>15;return activity(climber.id)===2&&unit(highGuard.id)!.health<1},'Combat after high tower ascent',60000);
 assert(climbedHigh,'Attack uses the lookout hatch before descending to the wall');
 await move(a,climber.id,-25,15);
 let exitedHigh=false;
 await until(()=>{const u=unit(climber.id);assert(u,'Climber survives withdrawal');exitedHigh ||= u.y>15;return u.y<1&&Math.hypot(u.x+25,u.z-15)<.15},'Withdrawal through high tower',60000);
 assert(exitedHigh,'Withdrawal climbs from wall to lookout hatch before exiting');
 console.log('PASS: lookout height transitions work for enemy tower entry and withdrawal.');
 await fresh('Isolated wall');
 const isolatedDefender=army(b)[0],ground=army(a)[0],isolated=await wall();
 assert.equal(await call(b,'moveToRoof',[[isolatedDefender.id],isolated.id,17,19]),'Committed');
 await until(()=>unit(isolatedDefender.id)!.y>7&&activity(isolatedDefender.id)===0,'Isolated setup');
 const tower=ownBuildings(b).find(v=>v.kind==='tower_stairwell')!;assert.equal(await call(b,'removeBuilding',[tower.id]),'Committed');
 assert.equal(await call(a,'attackUnits',[[ground.id],isolatedDefender.id]),'Failed','No ascent through a missing tower');
 assert.equal(unit(isolatedDefender.id)!.health,1);
 console.log('PASS: disconnected walls remain unreachable instead of taking damage from the ground.');
 await call(a,'leaveGame');await call(b,'leaveGame');
}finally{strongholdNetwork.disconnect();for(const c of clients)c.disconnect()}
