import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri=process.env.SPACETIMEDB_URI??'http://127.0.0.1:3399',database=process.env.SPACETIMEDB_DATABASE??'stronghold-archers';
const clients:DbConnection[]=[];
const sleep=(ms:number)=>new Promise(r=>setTimeout(r,ms));
async function until(f:()=>boolean,label:string,ms=45000){const end=Date.now()+ms;while(!f()){assert(Date.now()<end,label);await sleep(20)}}
async function connect(){return new Promise<DbConnection>((resolve,reject)=>DbConnection.builder().withUri(uri).withModuleName(database).onConnect(db=>{clients.push(db);db.subscriptionBuilder().onApplied(()=>resolve(db)).onError(c=>reject(c.event)).subscribe(['game','participant','army_unit','building','unit_profile','arrow_projectile'].map(t=>`SELECT * FROM ${t}`));}).onConnectError((_,e)=>reject(e)).build());}
function call(db:DbConnection,name:string,args:unknown[]=[]){return new Promise<string>((resolve,reject)=>{let done=false;const timer=setTimeout(()=>reject(new Error(`Timeout ${name}`)),8000);(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)]((c:ReducerEventContext)=>{if(done||c.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;done=true;clearTimeout(timer);resolve(c.event.status.tag)});(db.reducers as any)[name](...args)});}
try {
 const a=await connect(),b=await connect();
 let activeGame=0n;
 const own=(db:DbConnection)=>Array.from(a.db.armyUnit.iter()).filter(u=>u.owner.toHexString()===db.connectionId.toHexString()&&u.gameId===activeGame);
 const unit=(id:bigint)=>a.db.armyUnit.id.find(id);
 const activity=(id:bigint)=>a.db.unitProfile.unitId.find(id)?.activity;
 const buildings=(db:DbConnection)=>Array.from(db.db.building.iter()).filter(v=>v.owner.toHexString()===db.connectionId.toHexString()&&v.gameId===activeGame);
 const arrows=()=>Array.from(a.db.arrowProjectile.iter());
 const near=(id:bigint,x:number,z:number)=>!!unit(id)&&Math.hypot(unit(id)!.x-x,unit(id)!.z-z)<.15;
 async function move(db:DbConnection,id:bigint,x:number,z:number){assert.equal(await call(db,'setTargets',[[id],x,z]),'Committed');}
 async function fresh(label:string){
  await call(a,'leaveGame');await call(b,'leaveGame');
  await until(()=>!Array.from(a.db.participant.iter()).some(p=>p.connectionId.toHexString()===a.connectionId.toHexString())&&!Array.from(b.db.participant.iter()).some(p=>p.connectionId.toHexString()===b.connectionId.toHexString()),'Previous participation removed');
  const title=`Ranged ${label} ${Date.now()}`;
  assert.equal(await call(a,'createGame',[title]),'Committed');
  await until(()=>{const game=Array.from(a.db.game.iter()).find(g=>g.title===title);return !!game&&Array.from(a.db.participant.iter()).some(p=>p.connectionId.toHexString()===a.connectionId.toHexString()&&p.gameId===game.id)},'New game membership');
  activeGame=Array.from(a.db.game.iter()).find(g=>g.title===title)!.id;
  assert.equal(await call(b,'joinGame',[activeGame]),'Committed');
  await until(()=>own(a).length===1&&own(b).length===1&&Array.from(b.db.participant.iter()).some(p=>p.connectionId.toHexString()===b.connectionId.toHexString()&&p.gameId===activeGame),'Starting armies');
  await move(a,own(a)[0].id,-60,45);await move(b,own(b)[0].id,60,45);
 }
 async function recruit(db:DbConnection,x:number,kind='archer'){
  let barracks=buildings(db).find(v=>v.kind==='barracks');
  if(!barracks){assert.equal(await call(db,'build',[x,20,'barracks']),'Committed');await until(()=>buildings(db).some(v=>v.kind==='barracks'),'Barracks replicates');barracks=buildings(db).find(v=>v.kind==='barracks')!;}
  const old=new Set(own(db).map(u=>u.id));assert.equal(await call(db,'recruitUnit',[barracks.id,kind]),'Committed');
  await until(()=>own(db).some(u=>!old.has(u.id)),'Recruit replication');
  const u=own(db).find(u=>!old.has(u.id))!;assert.equal(a.db.unitProfile.unitId.find(u.id)!.kind,kind);return u;
 }
 await fresh('approach');
 const attacker=await recruit(a,-30),defender=await recruit(b,30);
 await move(a,attacker.id,-15,40);await move(b,defender.id,15,40);
 await until(()=>near(attacker.id,-15,40)&&near(defender.id,15,40),'Archers stage outside range');
 assert.equal(await call(a,'attackUnits',[[attacker.id],defender.id]),'Committed');
 await until(()=>arrows().some(p=>p.shooter===attacker.id),'Attacker shoots');
 const gap=Math.hypot(unit(attacker.id)!.x-unit(defender.id)!.x,unit(attacker.id)!.z-unit(defender.id)!.z);
 assert(gap>=13.5&&gap<=14.05,`Stops at bow range, not melee (${gap})`);
 assert(near(defender.id,15,40),'Automatic defender remains stationary');
 assert.equal(unit(defender.id)!.health,1,'No damage until a projectile lands');
 await until(()=>unit(defender.id)!.health<1&&unit(attacker.id)!.health<1,'Reciprocal arrow impacts');
 await move(a,attacker.id,-20,40);await until(()=>near(attacker.id,-20,40),'Explicit move cancels attack');
 assert(near(defender.id,15,40),'Defender does not pursue fleeing target');
 console.log('PASS: archer recruitment, approach stops at range, delayed replicated arrows, stationary defense and command override.');
 await fresh('elevation');
 const high=await recruit(a,-30),far=await recruit(b,30);
 await move(b,far.id,27,40);await until(()=>near(far.id,27,40),'Elevated target stages');
 assert.equal(await call(a,'build',[10,40,'tower_stairwell']),'Committed');const tower=buildings(a).find(v=>v.kind==='tower_stairwell')!;
 assert.equal(await call(a,'moveToRoof',[[high.id],tower.id,10,40]),'Committed');
 await until(()=>unit(high.id)!.y>7&&arrows().some(p=>p.shooter===high.id),'Height extends automatic bow range');
 assert(Math.hypot(unit(high.id)!.x-unit(far.id)!.x,unit(high.id)!.z-unit(far.id)!.z)>14.5);
 await until(()=>unit(far.id)!.health<1,'Arrows from tower hit ground targets');
 assert.equal(unit(high.id)!.health,1,'Ground archer cannot return fire beyond its own range');
 assert(!arrows().some(p=>p.shooter===far.id));
 console.log('PASS: tower height adds range and archers defend roofs without descending.');
 await fresh('isolated wall');
 const ground=await recruit(a,-30),roof=await recruit(b,30);
 await move(a,ground.id,40,41);await until(()=>near(ground.id,40,41),'Ground archer stages');
 assert.equal(await call(b,'build',[10,40,'tower_stairwell']),'Committed');
 assert.equal(await call(b,'buildWall',[13,41,17,41]),'Committed');
 const wall=buildings(b).find(v=>v.kind==='stone_wall'&&v.x===17)!;
 assert.equal(await call(b,'moveToRoof',[[roof.id],wall.id,17,41]),'Committed');await until(()=>unit(roof.id)!.y>7&&activity(roof.id)===0,'Wall archer settles');
 const access=buildings(b).find(v=>v.kind==='tower_stairwell')!;assert.equal(await call(b,'removeBuilding',[access.id]),'Committed');
 assert.equal(await call(a,'attackUnits',[[ground.id],roof.id]),'Committed');
 await until(()=>arrows().some(p=>p.shooter===ground.id),'Shoots isolated enemy wall from ground');
 assert(unit(ground.id)!.y<1,'Archer never climbs enemy fortifications');
 await until(()=>unit(roof.id)!.health<1,'Ground-to-roof arrow hit');
 console.log('PASS: enemy wall garrisons can be shot without a tower entry.');
 await fresh('blocked');
 const left=await recruit(a,-30),right=await recruit(b,30);
 assert.equal(await call(a,'buildWall',[1,69,1,93]),'Committed');
 await move(a,left.id,-5,81);await move(b,right.id,7,81);
 try {await until(()=>near(left.id,-5,81)&&near(right.id,7,81),'Blocked shot stages');} catch(error) {for(const id of [left.id,right.id]){const u=unit(id);console.log('Blocked stage',id.toString(),u&&{x:u.x,y:u.y,z:u.z,targetX:u.targetX,targetZ:u.targetZ,health:u.health},activity(id));}throw error;}await sleep(1800);
 assert.equal(unit(left.id)!.health,1);assert.equal(unit(right.id)!.health,1);
 assert(!arrows().some(p=>p.shooter===left.id||p.shooter===right.id),'Wall blocks low arrows');
 console.log('PASS: archers cannot shoot through stone walls.');
 await fresh('building');
 const raider=await recruit(a,-30);await move(a,raider.id,-10,80);await until(()=>near(raider.id,-10,80),'Building attacker stages');
 assert.equal(await call(b,'build',[16,80,'granary']),'Committed');const stock=buildings(b).find(v=>v.kind==='granary')!;
 assert.equal(await call(a,'attackBuilding',[[raider.id],stock.id]),'Committed');
 await until(()=>arrows().some(p=>p.shooter===raider.id&&p.building),'Arrows at building');
 const shooter=unit(raider.id)!;assert(Math.abs(shooter.x-(stock.x-3))>12,'Archer does not use building melee slots');
 await until(()=>a.db.building.id.find(stock.id)!.health<1,'Building hit by arrow');
 console.log('PASS: building attacks also use ranged projectiles.');
 await fresh('point blank');
 const close=await recruit(a,-30),old=await recruit(b,30);await move(b,old.id,60,60);
 const campX=old.x,campZ=old.z+.9;await move(a,close.id,campX,campZ);await until(()=>near(close.id,campX,campZ),'Archer camps near enemy barracks');
 const peasant=await recruit(b,30,'armed_peasant');
 await until(()=>unit(peasant.id)!.health<1,'Point blank arrow hits');
 assert(Math.abs(unit(peasant.id)!.health-.82)<.001,'Even at melee distance, damage is one arrow, never melee DPS');
 console.log('PASS: point-blank archers still deal only projectile damage.');
 await call(a,'leaveGame');await call(b,'leaveGame');
 assert(!arrows().some(p=>p.shooter===close.id),'Leaving releases projectiles');
 strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());await until(()=>snapshot().ready,'Browser ready');
 strongholdNetwork.send(JSON.stringify({action:'create',title:`Archer bridge ${Date.now()}`}));await until(()=>snapshot().units.length===1,'Browser lord');
 strongholdNetwork.send(JSON.stringify({action:'build',building:'barracks',x:10,z:18}));await until(()=>snapshot().buildings.some((v:any)=>v.kind==='barracks'),'Browser barracks');
 strongholdNetwork.send(JSON.stringify({action:'recruit_unit',building_id:snapshot().buildings.find((v:any)=>v.kind==='barracks').id,kind:'archer'}));await until(()=>snapshot().units.some((v:any)=>v.kind==='archer'),'Browser archer');
 assert(Array.isArray(snapshot().arrows),'Arrow snapshot exists');strongholdNetwork.send(JSON.stringify({action:'leave'}));await until(()=>snapshot().game_id===null,'Browser cleanup');
 console.log('PASS: browser archer recruitment and projectile snapshot.');
} finally {strongholdNetwork.disconnect();for(const c of clients)c.disconnect()}
