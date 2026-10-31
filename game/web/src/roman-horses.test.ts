import assert from 'node:assert/strict';
import {world,status,until} from './test-world';
const {db,game,own,build}=await world();const call=(n:string,...a:any[])=>status(db,c=>(db.reducers as any)[`on${n[0].toUpperCase()+n.slice(1)}`](c),()=>(db.reducers as any)[n](...a));const ok=(s:any,l:string)=>assert.equal(s.tag,'Committed',`${l}: ${s.value??s.tag}`);
try{for(const [k,x,z]of [['housing',23,-19],['housing',29,-19],['stockpile',-9,-23],['granary',-9,-35],['well',13,-19],['stable',24,22],['market',38,22],['barracks',-24,-24]]as [string,number,number][])ok(await build(k,x,z),k);
 const stable=own().find(b=>b.kind==='stable')!,bar=own().find(b=>b.kind==='barracks')!;const horses=()=>Array.from(db.db.stableHorse.iter()).filter(h=>h.stableId===stable.id);
 assert.equal((await call('sellHorses',stable.id,1)).tag,'Failed','no sale of decorative horses or unborn stock');assert.equal((await call('recruitUnit',bar.id,'light_cavalry')).tag,'Failed','needs adult horse');
 await until(()=>horses().filter(h=>h.age>=90).length>=2,'two adult horses',220000);const count=horses().length,gold=db.db.treasury.connectionId.find(db.connectionId)!.gold;
 ok(await call('sellHorses',stable.id,1),'sell adult');assert.equal(horses().length,count-1);assert.ok(db.db.treasury.connectionId.find(db.connectionId)!.gold>gold);
 ok(await call('recruitUnit',bar.id,'light_cavalry'),'mounted recruit');assert.equal(horses().length,count-2);assert.equal((await call('sellHorses',stable.id,8)).tag,'Failed','no overselling');console.log('PASS adult-only horse sale credits treasury, cavalry consumes one real horse, no overselling');
}finally{db.disconnect()}
