import assert from 'node:assert/strict';
import {world,status,until} from './test-world';
const {db,game,own,build}=await world();const call=(n:string,...args:any[])=>status(db,c=>(db.reducers as any)[`on${n[0].toUpperCase()+n.slice(1)}`](c),()=>(db.reducers as any)[n](...args));const ok=(s:any,l:string)=>assert.equal(s.tag,'Committed',`${l}: ${s.value??s.tag}`);
try{
 for(const [k,x,z]of [['housing',23,-19],['housing',29,-19],['housing',35,-19],['stockpile',-9,-23],['granary',-9,-35],['well',13,-19],['tool_smith',-24,-24],['vineyard',0,24],['vintners_workshop',-18,26],['olive_grove',24,22],['olive_press',40,24],['saw_pit',40,-43],['market',-24,42]]as [string,number,number][])ok(await build(k,x,z),k);
 const stock=()=>Array.from(db.db.workshopStock.iter()).filter(r=>r.gameId===game);const tools=own().find(b=>b.kind==='tool_smith')!;
 await until(()=>stock().some(s=>s.tools>0),'tool smith manufactures stored tools',75000);ok(await call('unassignPeasant',tools.id),'pause tool smith');console.log('PASS tool manufacture');
 await until(()=>stock().some(s=>s.grapes>0),'grapes physically delivered',90000);await until(()=>stock().some(s=>s.olives>0),'olives physically delivered',90000);console.log('PASS grape and olive harvest/delivery');
 await until(()=>stock().some(s=>s.wine>0),'wine press converts delivered grapes',120000);await until(()=>stock().some(s=>s.oil>0),'olive press converts delivered olives',120000);console.log('PASS wine and oil chains');
 const before=db.db.treasury.connectionId.find(db.connectionId)!.gold;const wine=stock().reduce((n,s)=>n+s.wine,0);ok(await call('sellWorkshopGoods',14,1),'sell wine');assert.ok(db.db.treasury.connectionId.find(db.connectionId)!.gold>before);assert.ok(stock().reduce((n,s)=>n+s.wine,0)<=wine);assert.equal((await call('sellWorkshopGoods',14,120)).tag,'Failed','cannot sell nonexistent goods');
 ok(await call('saveGame',`Wine tools oil ${Date.now()}`),'save goods');const slot=Array.from(db.db.saveCatalog.iter()).sort((a,b)=>Number(b.id-a.id))[0];ok(await call('loadGame',slot.id),'load goods');const loaded=Array.from(db.db.participant.iter()).find(p=>p.connectionId.toHexString()===db.connectionId.toHexString())!.gameId;assert.ok(Array.from(db.db.workshopStock.iter()).some(s=>s.gameId===loaded&&s.oil>0));
 console.log('PASS: staffed tool manufacture, actual grape/olive hauling and press inputs, wine/oil stock, sale proceeds, no overselling, saved workshop goods.');
}finally{db.disconnect()}
