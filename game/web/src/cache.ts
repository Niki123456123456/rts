import type { TableCache } from '@clockworklabs/spacetimedb-sdk';
type Operation=Parameters<TableCache['applyOperations']>[0][number];
const serialized=(row:unknown)=>JSON.stringify(row,(_key,value)=>typeof value==='bigint'?value.toString():value);
// All replicated game tables use one of these primary-key column names.
const keyOf=(row:Record<string,unknown>)=>{
 const column=['id','unitId','buildingId','connectionId','gameId','scheduledId'].find(key=>key in row);
 return column===undefined?undefined:`${column}:${serialized(row[column])}`;
};
/** Preserve net subscription reference counts, canceling intermediate row values.
 * SDK 1.1.0 keys storage by the complete serialized row, even for primary-key
 * tables. Resolve deletes through the actual cached primary-key row so replacing
 * a row or removing a dead unit cannot leave an old serialized version behind.
 */
export function netOperations(operations:Operation[],current?:unknown[]):Operation[]{
 const rows=new Map<string,{op:Operation;count:number}>();
 for(const op of operations){const previous=rows.get(op.rowId);rows.set(op.rowId,{op,count:(previous?.count??0)+(op.type==='insert'?1:-1)});}
 const present=current===undefined?undefined:new Map(current.map(row=>[keyOf(row as Record<string,unknown>),row]));
 const result:Operation[]=[];
 for(const {op,count} of rows.values()){
  let canonical=op;
  if(count<0&&present){const key=keyOf(op.row);if(key!==undefined){const row=present.get(key);if(row===undefined)continue;canonical={...op,row,rowId:serialized(row)};}}
  for(let i=0;i<Math.abs(count);i++)result.push({...canonical,type:count>0?'insert':'delete'});
 }
 return result;
}
const installed=new WeakSet<object>();
export function prepareCaches(tables:object){
 for(const [key,descriptor] of Object.entries(Object.getOwnPropertyDescriptors(Object.getPrototypeOf(tables)))){
  if(!descriptor.get)continue;
  const table=Reflect.get(tables,key) as {tableCache:TableCache};
  const cache=table.tableCache;
  if(!cache||installed.has(cache))continue;
  const apply=cache.applyOperations.bind(cache);
  cache.applyOperations=(operations,context)=>apply(netOperations(operations,cache.iter()),context);
  installed.add(cache);
 }
}
