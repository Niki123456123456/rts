import assert from 'node:assert/strict';
import {world,status,until,delay} from './test-world';
const w=await world(),{db,game,own,build}=w;
const call=(name:string,...args:any[])=>status(db,c=>(db.reducers as any)[`on${name[0].toUpperCase()+name.slice(1)}`](c),()=>(db.reducers as any)[name](...args));
const committed=(s:any,label:string)=>assert.equal(s.tag,'Committed',`${label}: ${s.value??s.tag}`);
const rows=(table:string)=>Array.from((db.db as any)[table].iter()).filter((r:any)=>r.gameId===game) as any[];
try {
 for(const [k,x,z] of [['housing',23,-19],['housing',29,-19],['stockpile',-9,-23],['granary',-9,-35],['wheat_farm',0,24],['windmill',-18,26],['bakery',-30,26],['stable',24,22],['well',13,-19],['barracks',-25,-23]] as [string,number,number][])committed(await build(k,x,z),k);
 const stable=own().find(b=>b.kind==='stable')!,mill=own().find(b=>b.kind==='windmill')!,bakery=own().find(b=>b.kind==='bakery')!,bar=own().find(b=>b.kind==='barracks')!,granary=own().find(b=>b.kind==='granary')!;
 committed(await call('unassignPeasant',mill.id),'close mill');committed(await call('unassignPeasant',bakery.id),'close bakery');
 assert.equal((await call('recruitUnit',bar.id,'light_cavalry')).tag,'Failed','no free cavalry without a grown horse');
 assert.equal((await call('recruitUnit',bar.id,'ballista')).tag,'Failed','artillery requires manufacture');
 await until(()=>rows('industryStock').some(s=>s.wheat>=5),'grain physically delivered',100000);console.log('PASS grain delivery');
 assert.equal(rows('industryStock').reduce((n,s)=>n+s.flour,0),0,'closed mill creates no flour');
 committed(await call('assignPeasant',mill.id),'open mill');
 await until(()=>rows('industryStock').some(s=>s.flour>=5),'mill converts delivered grain into delivered flour',90000);console.log('PASS flour delivery');
 committed(await call('assignPeasant',bakery.id),'open bakery');
 let delivered=false;const before=db.db.granaryStock.buildingId.find(granary.id)!.bread;const watch=setInterval(()=>{const bread=db.db.granaryStock.buildingId.find(granary.id)?.bread??0;if(bread>before)delivered=true;},20);
 try{await until(()=>delivered,'bakery delivers real bread',90000);}finally{clearInterval(watch)}console.log('PASS bread delivery');
 await until(()=>rows('stableHorse').some(h=>h.stableId===stable.id&&h.age>=90),'foal becomes a usable adult',180000);
 const horses=rows('stableHorse').length;const material=db.db.playerResources.connectionId.find(db.connectionId)!.wood;committed(await call('recruitUnit',bar.id,'light_cavalry'),'recruit mounted unit');assert.equal(rows('stableHorse').length,horses-1);assert.equal(material-db.db.playerResources.connectionId.find(db.connectionId)!.wood,8);
 const rider=rows('armyUnit').find(u=>db.db.unitProfile.unitId.find(u.id)?.kind==='light_cavalry');assert.ok(rider);console.log('PASS adult horse consumed by cavalry');
 committed(await call('unassignPeasant',stable.id),'close stable');const count=rows('stableHorse').length;await delay(47000);assert.equal(rows('stableHorse').length,count,'closed stable stops births');
 const saved=rows('stableHorse').map(h=>h.age);committed(await call('saveGame',`Industry ${Date.now()}`),'save');const slot=Array.from(db.db.saveCatalog.iter()).sort((a,b)=>Number(b.id-a.id))[0];assert.ok(slot);committed(await call('loadGame',slot.id),'load');
 const newGame=Array.from(db.db.participant.iter()).find(p=>p.connectionId.toHexString()===db.connectionId.toHexString())!.gameId;assert.notEqual(newGame,game);
 const restored=Array.from(db.db.stableHorse.iter()).filter(h=>h.gameId===newGame);assert.equal(restored.length,saved.length);assert.deepEqual(restored.map(h=>h.age).sort((a,b)=>a-b),saved.sort((a,b)=>a-b));assert.ok(restored.every(h=>Array.from(db.db.building.iter()).some(b=>b.id===h.stableId&&b.gameId===newGame)));
 assert.ok(Array.from(db.db.industryStock.iter()).some(s=>s.gameId===newGame),'saved grain/flour restore with remapped warehouse IDs');
 console.log('PASS: grain → flour → bread with workers and deliveries; foals mature, real horses fund cavalry; closure stops births; industry stocks and horses survive save/load.');
}finally{db.disconnect()}
