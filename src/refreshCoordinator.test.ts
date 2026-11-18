import {createNativeHistoryTracker} from './nativeHistory';
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createRefreshCoordinator} from './refreshCoordinator';

function deferred<T>() {
  let resolve!: (value:T) => void;
  let reject!: (error:unknown) => void;
  const promise = new Promise<T>((yes,no) => {resolve=yes;reject=no;});
  return {promise,resolve,reject};
}
const tick = async () => {await Promise.resolve();await Promise.resolve();};

test('a save/event overtaking a slow read never publishes stale state or history', async () => {
  const old=deferred<{rating:number;history:number[]}>(), fresh=deferred<{rating:number;history:number[]}>();
  const reads=[old,fresh];let count=0;
  const applied:unknown[]=[];
  const refresh=createRefreshCoordinator(()=>reads[count++].promise, value=>applied.push(value));
  const initial=refresh();await tick();
  const saved=refresh();const event=refresh();
  assert.equal(count,1,'overlapping refreshes do not start parallel database reads');
  old.resolve({rating:1,history:[1]});await tick();
  assert.deepEqual(applied,[],'obsolete result must not change UI or record history');
  assert.equal(count,2,'the burst schedules one follow-up read');
  const latest={rating:5,history:[1,2]};fresh.resolve(latest);
  assert.deepEqual(await Promise.all([initial,saved,event]),[latest,latest,latest]);
  assert.deepEqual(applied,[latest]);
});

test('obsolete failures are ignored; current failures reject all waiters and permit retry', async () => {
  const old=deferred<string>(), fresh=deferred<string>();let count=0;
  const applied:string[]=[];
  const refresh=createRefreshCoordinator(()=>[old,fresh][count++].promise,v=>applied.push(v));
  const first=refresh();await tick();const second=refresh();
  old.reject(Error('obsolete'));await tick();fresh.resolve('current');
  assert.deepEqual(await Promise.all([first,second]),['current','current']);
  assert.deepEqual(applied,['current']);
  let fail=true;
  const retry=createRefreshCoordinator(async()=>{if(fail)throw Error('current failure');return 'recovered';},v=>applied.push(v));
  const a=retry(),b=retry();
  await Promise.all([assert.rejects(a,/current failure/),assert.rejects(b,/current failure/)]);
  fail=false;assert.equal(await retry(),'recovered');
});

test('requests arriving during the follow-up read wait for the newest snapshot', async () => {
  const reads=[deferred<string>(),deferred<string>(),deferred<string>()];let count=0;
  const applied:string[]=[];
  const refresh=createRefreshCoordinator(()=>reads[count++].promise,v=>applied.push(v));
  const a=refresh();await tick();const b=refresh();reads[0].resolve('old library');await tick();
  const c=refresh();reads[1].resolve('intermediate');await tick();reads[2].resolve('current library');
  assert.deepEqual(await Promise.all([a,b,c]),Array(3).fill('current library'));
  assert.deepEqual(applied,['current library']);
});

test('a refresh triggered while applying is not lost', async () => {
  let count=0;const applied:number[]=[];
  const refresh=createRefreshCoordinator(async()=>++count,value=>{applied.push(value);if(value===1)void refresh();});
  assert.equal(await refresh(),2);assert.deepEqual(applied,[1,2]);
});

test('native history baselines libraries and preserves ordered new entries across undo/redo',()=>{
 const observe=createNativeHistoryTracker();
 const entry=(id:number)=>({id,label:`Change ${id}`});
 assert.deepEqual(observe({root:'A',undo_entries:[entry(1)]}),[]);
 assert.deepEqual(observe({root:'A',undo_entries:[entry(1),entry(2),entry(2),entry(3)]}),[entry(2),entry(3)]);
 assert.deepEqual(observe({root:'A',undo_entries:[entry(1),entry(2)]}),[]);
 assert.deepEqual(observe({root:'A',undo_entries:[entry(1),entry(2),entry(3)]}),[entry(3)]);
 assert.deepEqual(observe({root:'B',undo_entries:[entry(1),entry(9)]}),[]);
 assert.deepEqual(observe({root:'B',undo_entries:[entry(1),entry(9),entry(10)]}),[entry(10)]);
});
test('obsolete native history cannot advance the accepted history baseline',async()=>{
 const observe=createNativeHistoryTracker(),stale=deferred<{root:string;undo_entries:{id:number;label:string}[]}>();
 const applied:number[]=[];let reads=0;
 observe({root:'A',undo_entries:[]});
 const fresh={root:'A',undo_entries:[{id:2,label:'Saved'}]};
 const refresh=createRefreshCoordinator(()=>++reads===1?stale.promise:Promise.resolve(fresh),snapshot=>{applied.push(...observe(snapshot).map(e=>e.id));});
 const first=refresh();await tick();const second=refresh();
 stale.resolve({root:'A',undo_entries:[{id:1,label:'Obsolete'}]});
 await Promise.all([first,second]);assert.deepEqual(applied,[2]);
 assert.deepEqual(observe(fresh),[]);
});
