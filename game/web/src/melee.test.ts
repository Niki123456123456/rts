import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri=process.env.SPACETIMEDB_URI??'http://127.0.0.1:3300', database=process.env.SPACETIMEDB_DATABASE??'stronghold-melee';
const clients:DbConnection[]=[];
const sleep=(ms:number)=>new Promise(r=>setTimeout(r,ms));
async function until(f:()=>boolean,label:string,ms=45000){const end=Date.now()+ms;while(!f()){assert(Date.now()<end,label);await sleep(25)}}
async function connect(){return new Promise<DbConnection>((resolve,reject)=>DbConnection.builder().withUri(uri).withModuleName(database).onConnect(db=>{clients.push(db);db.subscriptionBuilder().onApplied(()=>resolve(db)).onError(c=>reject(c.event)).subscribe(['game','participant','army_unit','building','building_rotation','unit_profile','player_resources','worker'].map(t=>`SELECT * FROM ${t}`));}).onConnectError((_,e)=>reject(e)).build());}
function call(db:DbConnection,name:string,args:unknown[]=[]){return new Promise<string>((resolve,reject)=>{let done=false;const timer=setTimeout(()=>reject(new Error(`Timeout ${name}`)),8000);(db.reducers as any)['on'+name[0].toUpperCase()+name.slice(1)]((c:ReducerEventContext)=>{if(done||c.event.callerConnectionId?.toHexString()!==db.connectionId.toHexString())return;done=true;clearTimeout(timer);resolve(c.event.status.tag)});(db.reducers as any)[name](...args)});}
try {
 const a=await connect(),b=await connect();
 let activeGame=0n;
 const rows=()=>Array.from(a.db.armyUnit.iter());
 const own=(db:DbConnection)=>rows().filter(u=>u.owner.toHexString()===db.connectionId.toHexString()&&u.gameId===activeGame);
 const unit=(id:bigint)=>a.db.armyUnit.id.find(id);
 const activity=(id:bigint)=>a.db.unitProfile.unitId.find(id)?.activity;
 const near=(id:bigint,x:number,z:number)=>{const u=unit(id);return !!u&&Math.hypot(u.x-x,u.z-z)<.12};
 async function fresh(label:string){
  assert.equal(await call(a,'leaveGame'),'Committed');assert.equal(await call(b,'leaveGame'),'Committed');
  const title=`Melee ${label} ${Date.now()}`;
  assert.equal(await call(a,'createGame',[title]),'Committed');
  await until(()=>{const game=Array.from(a.db.game.iter()).find(g=>g.title===title);return !!game&&Array.from(a.db.participant.iter()).some(p=>p.connectionId.toHexString()===a.connectionId.toHexString()&&p.gameId===game.id)},'New game membership');
  const game=Array.from(a.db.game.iter()).find(g=>g.title===title)!.id;
  activeGame=game;assert.equal(await call(b,'joinGame',[game]),'Committed');await until(()=>own(a).length===1&&own(b).length===1,'Armies replicate');for(const [db,x] of [[a,-121],[b,121]] as const) for(let i=0;i<3;i++) assert.equal(await call(db,'build',[x,45+i*10,'housing']),'Committed');assert.equal(await call(a,'build',[-71,65,'stockpile']),'Committed');assert.equal(await call(b,'build',[71,65,'stockpile']),'Committed');return game;
 }
 async function move(db:DbConnection,id:bigint,x:number,z:number){assert.equal(await call(db,'setTargets',[[id],x,z]),'Committed');}
 async function recruit(db:DbConnection,x:number,n:number){
  assert.equal(await call(db,'build',[x,-20,'barracks']),'Committed');
  const barracks=Array.from(db.db.building.iter()).find(v=>v.owner.toHexString()===db.connectionId.toHexString()&&v.kind==='barracks')!;
  for(let i=0;i<n;i++)assert.equal(await call(db,'recruitUnit',[barracks.id,'armed_peasant']),'Committed');
  await until(()=>own(db).length===n+1,'Recruits replicate');return own(db).filter(u=>u.number>1);
 }
 await fresh('group');
 const lordA=own(a)[0],lordB=own(b)[0];await move(b,lordB.id,60,-30);
 const allies=await recruit(a,-30,2),enemies=await recruit(b,30,2);
 const attackers=[lordA,...allies];
 for(let i=0;i<attackers.length;i++)await move(a,attackers[i].id,-10-i*.8,20);
 for(let i=0;i<enemies.length;i++)await move(b,enemies[i].id,10+i*.8,20);
 await until(()=>attackers.every((u,i)=>near(u.id,-10-i*.8,20))&&enemies.every((u,i)=>near(u.id,10+i*.8,20))&&near(lordB.id,60,-30),'Armies stage outside sensing radius');
 assert.equal(await call(a,'attackUnits',[[lordA.id],lordA.id]),'Failed','Friendly fire rejected');
 assert.equal(await call(a,'attackUnits',[[lordB.id],enemies[0].id]),'Failed','Foreign command rejected');
 assert.equal(await call(a,'attackUnits',[[lordA.id],999999n]),'Failed','Unknown target rejected');
 const foreign=await connect();await call(foreign,'createGame',[`Other game ${Date.now()}`]);
 const outsider=Array.from(foreign.db.armyUnit.iter()).find(u=>u.owner.toHexString()===foreign.connectionId.toHexString())!;
 assert.equal(await call(a,'attackUnits',[[lordA.id],outsider.id]),'Failed','Cross-game target rejected');
 await call(foreign,'leaveGame');foreign.disconnect();
 assert.equal(await call(a,'attackUnits',[attackers.map(u=>u.id),enemies[0].id]),'Committed');
 await until(()=>attackers.some(u=>activity(u.id)===2)&&enemies.some(u=>activity(u.id)===2),'Both sides fight');
 for(const u of attackers.filter(u=>activity(u.id)===2))assert(enemies.some(e=>unit(e.id)&&Math.hypot(unit(u.id)!.x-unit(e.id)!.x,unit(u.id)!.z-unit(e.id)!.z)<=1.06),'Melee only in range');
 await until(()=>attackers.some(u=>(unit(u.id)?.health??1)<1)&&enemies.some(u=>(unit(u.id)?.health??1)<1),'Both sides take damage');
 await until(()=>enemies.every(u=>!unit(u.id)),'One click defeats entire connected enemy group');
 assert(attackers.some(u=>!!unit(u.id)),'Surviving attackers remain');assert.equal(unit(lordB.id)!.health,1,'Distant enemy is outside clicked group');
 for(const e of enemies)assert.equal(a.db.unitProfile.unitId.find(e.id),undefined,'Casualties release profiles');
 console.log('PASS: group targeting, bilateral melee, range, health, death and ownership.');
 await fresh('automatic');
 const defender=own(a)[0],remote=own(b)[0];await move(b,remote.id,60,-30);
 const raider=(await recruit(b,30,1))[0];await move(b,raider.id,4,-1.05);
 await until(()=>activity(defender.id)===2&&activity(raider.id)===2,'Automatic attack and counterattack');
 await until(()=>!unit(raider.id),'Stronger lord defeats invading peasant');
 await until(()=>near(defender.id,defender.x,defender.z)&&activity(defender.id)===0,'Automatic defender returns home');
 assert(unit(defender.id)!.health>0&&unit(defender.id)!.health<1);
 console.log('PASS: autonomous defense, reciprocal animations and return home.');
 await fresh('leash');
 const guard=own(a)[0],runner=own(b)[0];await move(b,runner.id,5,-1.05);
 await until(()=>activity(guard.id)===1&&Math.hypot(unit(guard.id)!.x-guard.x,unit(guard.id)!.z-guard.z)>.3,'Guard acquires nearby enemy');
 await move(b,runner.id,45,-1.05);let furthest=0;
 await until(()=>{const u=unit(guard.id)!;furthest=Math.max(furthest,Math.hypot(u.x-guard.x,u.z-guard.z));return unit(runner.id)!.x>25&&near(guard.id,guard.x,guard.z)&&activity(guard.id)===0},'Pursuit ends and guard returns',60000);
 assert(furthest>4&&furthest<=16.5,`Pursuit remains bounded (${furthest})`);
 console.log('PASS: moving targets and bounded pursuit.');
 await fresh('cancel');
 const mover=own(a)[0],bait=own(b)[0];await move(b,bait.id,5,-1.05);
 await until(()=>activity(mover.id)===1&&unit(mover.id)!.x>mover.x+.3,'Automatic pursuit begins');
 await move(a,mover.id,-25,15);
 await until(()=>near(mover.id,-25,15)&&activity(mover.id)===0,'Explicit move cancels automatic engagement');
 console.log('PASS: explicit commands override automatic combat.');
 await fresh('wall');
 const wa=own(a)[0],wb=own(b)[0];
 assert.equal(await call(a,'buildWall',[1,-1,1,-1]),'Committed');
 assert.equal(await call(a,'attackUnits',[[wa.id],wb.id]),'Committed');
 await until(()=>{for(const id of [wa.id,wb.id]){const u=unit(id);if(u)assert(!(u.x>-.26&&u.x<2.26&&u.z>-2.26&&u.z<.26),'Fighters navigate around wall');}return activity(wa.id)===2&&activity(wb.id)===2},'Enemy reached around wall');
 console.log('PASS: unit attacks reuse obstacle-aware navigation.');
 await fresh('roof');
 const groundLord=own(a)[0],roofLord=own(b)[0];
 assert.equal(await call(b,'build',[20,20,'tower_stairwell']),'Committed');
 const tower=Array.from(b.db.building.iter()).find(v=>v.owner.toHexString()===b.connectionId.toHexString()&&v.kind==='tower_stairwell')!;
 assert.equal(await call(b,'moveToRoof',[[roofLord.id],tower.id,20,20]),'Committed');
 await until(()=>unit(roofLord.id)!.y>3&&activity(roofLord.id)===0,'Defender reaches roof');
 assert.equal(await call(a,'attackUnits',[[groundLord.id],roofLord.id]),'Committed','Enemy roof reached through its doorway');
 await until(()=>activity(groundLord.id)===2&&activity(roofLord.id)===2,'Roof combat after ascent');
 assert(unit(groundLord.id)!.y>3,'No melee damage from below');
 console.log('PASS: roof combat uses the tower entry.');
 assert.equal(await call(a,'leaveGame'),'Committed');assert.equal(await call(b,'leaveGame'),'Committed');
 strongholdNetwork.send(JSON.stringify({action:'connect',uri,database}));const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());await until(()=>snapshot().ready,'Browser ready');
 strongholdNetwork.send(JSON.stringify({action:'create',title:`Melee bridge ${Date.now()}`}));await until(()=>snapshot().units.length===1,'Browser lord');
 await call(b,'joinGame',[BigInt(snapshot().game_id)]);await until(()=>snapshot().units.length===2,'Browser sees opponent');
 const mine=snapshot().units.find((u:any)=>u.owned),enemy=snapshot().units.find((u:any)=>!u.owned);
 strongholdNetwork.send(JSON.stringify({action:'attack_units',unit_ids:[mine.id],target_unit_id:enemy.id}));
 await until(()=>snapshot().units.some((u:any)=>u.activity===2&&u.health<1),'Browser attack action, animations and damage');
 strongholdNetwork.send(JSON.stringify({action:'leave'}));await until(()=>snapshot().game_id===null,'Browser cleanup');await call(b,'leaveGame');
 console.log('PASS: browser unit attack command and replicated combat snapshots.');
} finally {strongholdNetwork.disconnect();for(const c of clients)c.disconnect()}
