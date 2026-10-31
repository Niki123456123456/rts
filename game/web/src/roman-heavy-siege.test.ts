import assert from 'node:assert/strict';
import {world,status,until} from './test-world';
const {db,game,own,build}=await world();
const call=(n:string,...a:any[])=>status(db,c=>(db.reducers as any)[`on${n[0].toUpperCase()+n.slice(1)}`](c),()=>(db.reducers as any)[n](...a));const ok=(s:any,l:string)=>assert.equal(s.tag,'Committed',`${l}: ${s.value??s.tag}`);
try{
 for(const [k,x,z]of [['housing',23,-19],['housing',29,-19],['stockpile',-9,-23],['granary',-9,-35],['well',13,-19],['siege_workshop',-24,-24],['research_center',24,22],['market',38,22]]as [string,number,number][])ok(await build(k,x,z),k);
 const keep=own().find(b=>b.kind==='keep')!,factory=own().find(b=>b.kind==='siege_workshop')!,center=own().find(b=>b.kind==='research_center')!;
 assert.equal((await call('queueSiege',factory.id,'siege_tower')).tag,'Failed','research required');ok(await call('startResearch',center.id,'weapons'),'research');
 await until(()=>db.db.researchState.keepId.find(keep.id)?.weapons===1,'research unlock',90000);
 ok(await call('marketTrade',2,35,false),'sell iron');ok(await call('marketTrade',1,35,false),'sell stone');ok(await call('marketTrade',0,55,true),'buy timber');
 for(const [kind,crew] of [['stone_ballista',2],['covered_ram',3],['siege_tower',3]] as [string,number][]){
 await until(()=>db.db.keepPopulation.buildingId.find(keep.id)!.arrived>=crew+2,'available crew',65000);ok(await call('queueSiege',factory.id,kind),kind);
 await until(()=>Array.from(db.db.armyUnit.iter()).some(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind===kind),kind+' complete',80000);
 const unit=Array.from(db.db.armyUnit.iter()).find(u=>u.gameId===game&&db.db.unitProfile.unitId.find(u.id)?.kind===kind)!;ok(await call('setTargets',[unit.id],-24-['stone_ballista','covered_ram','siege_tower'].indexOf(kind)*6,-42),'clear exit');console.log('PASS researched manufacture '+kind);
 }
}finally{db.disconnect()}
