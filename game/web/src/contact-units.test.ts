import assert from 'node:assert/strict';
import {world,status,until,delay} from './test-world';
const w=await world(),{db,game,build,own}=w;
const call=(name:string,...args:any[])=>status(db,c=>(db.reducers as any)[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>(db.reducers as any)[name](...args));
const stock=()=>db.db.playerResources.connectionId.find(db.connectionId)!;
try {
 assert.deepEqual([stock().wood,stock().stone,stock().iron],[120,120,120]);assert.equal(db.db.treasury.connectionId.find(db.connectionId)!.gold,120n);
 assert.equal((await build('housing',23,-19)).tag,'Committed');const first=own().find(b=>b.kind==='housing')!;
 // Original housing bounds are 4.286015 by 4.925744 m, not its 6x6 raster plot.
 const width=4.2860150146484375;
 assert.equal((await build('housing',first.x+width,first.z)).tag,'Committed');const homes=own().filter(b=>b.kind==='housing').sort((a,b)=>a.x-b.x);
 assert.ok(Math.abs(homes[1].x-homes[0].x-width)<.002,'houses touch at their visible wall edges');
 assert.equal((await build('housing',first.x,first.z)).tag,'Failed','actual overlapping buildings remain forbidden');
 assert.equal((await build('barracks',-25,-23)).tag,'Committed');const barracks=own().find(b=>b.kind==='barracks')!;
 assert.equal((await build('stable',24,22)).tag,'Committed');
 await until(()=>Array.from(db.db.stableHorse.iter()).some(h=>h.gameId===game&&h.age>=90),'stable supplies a grown cavalry horse',180000);
 await until(()=>db.db.keepPopulation.buildingId.find(own().find(b=>b.kind==='keep')!.id)!.arrived>=2,'free peasants arrive',15000);
 const before=stock();const peasants=db.db.keepPopulation.buildingId.find(own().find(b=>b.kind==='keep')!.id)!.arrived;
 assert.equal((await call('recruitUnit',barracks.id,'light_swordsman')).tag,'Committed');
 assert.equal((await call('recruitUnit',barracks.id,'light_cavalry')).tag,'Committed');
 assert.equal(before.wood-stock().wood,10);assert.equal(before.iron-stock().iron,7);
 assert.ok(db.db.keepPopulation.buildingId.find(own().find(b=>b.kind==='keep')!.id)!.arrived<=peasants,'recruits consume free peasants');
 const find=(kind:string)=>Array.from(db.db.armyUnit.iter()).find(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind===kind)!;
 const sword=find('light_swordsman'),rider=find('light_cavalry');assert.ok(sword&&rider);
 // Move across unobstructed grass, far enough to measure cruising speed.
 assert.equal((await call('setTargets',[sword.id],sword.x-35,sword.z)).tag,'Committed');
 assert.equal((await call('setTargets',[rider.id],rider.x-35,rider.z-2)).tag,'Committed');
 await delay(2200);let s=find('light_swordsman'),r=find('light_cavalry');const s0={x:s.x,z:s.z},r0={x:r.x,z:r.z};await delay(2000);s=find('light_swordsman');r=find('light_cavalry');
 const foot=Math.hypot(s.x-s0.x,s.z-s0.z),horse=Math.hypot(r.x-r0.x,r.z-r0.z);assert.ok(foot>3&&horse>foot*1.4,`horse ${horse.toFixed(2)}m versus foot ${foot.toFixed(2)}m`);
 assert.equal((await call('setCivicPolicy',4,7)).tag,'Committed');const gold=db.db.treasury.connectionId.find(db.connectionId)!.gold;await until(()=>db.db.treasury.connectionId.find(db.connectionId)!.gold>gold,'tax round credits actual gold',35000);
 console.log('PASS: 120 starting stocks and sesterces, direct visible-edge construction without overlap, both new barracks units with real costs and peasants, cavalry cruising speed, real recurring tax income.');
}finally{db.disconnect()}
