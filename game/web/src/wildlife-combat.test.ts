import assert from 'node:assert/strict';
import {world,status,until} from './test-world';
const w=await world();const {db,game,lord}=w;
try {
 const deer=Array.from(db.db.wildlife.iter()).filter(a=>a.gameId===game&&a.species===0).sort((a,b)=>Math.hypot(a.x-lord().x,a.z-lord().z)-Math.hypot(b.x-lord().x,b.z-lord().z))[0];
 assert.ok(deer);assert.equal((await status(db,c=>db.reducers.onHuntAnimal(c),()=>db.reducers.huntAnimal([lord().id],deer.id))).tag,'Committed');
 await until(()=>db.db.wildlife.id.find(deer.id)?.health===0,'lord descends from palace, pursues and kills moving animal',100000);
 assert.ok(db.db.wildlife.id.find(deer.id)!.meat>0,'actual carcass remains');
 assert.equal((await w.build('barracks',40,-12)).tag,'Committed');
 const barracks=w.own().find(b=>b.kind==='barracks')!;
 const stock=()=>db.db.playerResources.connectionId.find(db.connectionId)!;
 await until(()=>Array.from(db.db.keepPopulation.iter()).some(p=>p.gameId===game&&p.arrived>=1),'free peasant available',45000);
 const before={wood:stock().wood,stone:stock().stone,iron:stock().iron};
 assert.equal((await status(db,c=>db.reducers.onRecruitUnit(c),()=>db.reducers.recruitUnit(barracks.id,'armed_peasant'))).tag,'Committed');
 assert.deepEqual({wood:stock().wood,stone:stock().stone,iron:stock().iron},{...before,wood:before.wood-1},'peasant soldier costs exactly one wood');
 console.log('PASS: melee pursuit from palace roof kills real wildlife; carcass persists; peasant recruitment costs exactly one wood.');
}finally{db.disconnect()}
