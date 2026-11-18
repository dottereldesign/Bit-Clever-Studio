import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { MediaQueue } from './mediaQueue';

const flush = () => new Promise(resolve => setTimeout(resolve, 0));
test('shares an active decode across remounts without letting one cancellation break the other', async () => {
  const queue = new MediaQueue();
  let calls = 0, finish!: (value: string) => void;
  const decode = () => { calls++; return new Promise<string>(resolve => { finish = resolve; }); };
  const abort = new AbortController();
  const first = queue.request('same', true, decode, abort.signal);
  const cancelled = assert.rejects(first, /Cancelled/);
  abort.abort();
  const second = queue.request('same', true, decode);
  await flush();
  assert.equal(calls, 1);
  finish('cached.jpg');
  assert.equal(await second, 'cached.jpg');
  await cancelled;
  await flush();
  assert.deepEqual(queue.getProgress(), { total: 1, completed: 1, failed: 0, pending: 0 });
});

test('bounds concurrency, skips obsolete queued cards, and continues after a decode failure', async () => {
  const queue = new MediaQueue();
  const finishes: Array<(value: string) => void> = [];
  const running = Array.from({ length: 3 }, (_, i) => queue.request(String(i), true, () => new Promise<string>(resolve => finishes.push(resolve))));
  const abort = new AbortController();
  const obsolete = queue.request('obsolete', true, async () => { throw Error('Must not decode'); }, abort.signal);
  const cancelled = assert.rejects(obsolete, /Cancelled/);
  abort.abort();
  const broken = queue.request('broken', true, async () => { throw Error('Unsupported'); });
  const failure = assert.rejects(broken, /Unsupported/);
  const next = queue.request('next', true, async () => 'next.jpg');
  await flush();
  assert.equal(finishes.length, 3);
  finishes.forEach(finish => finish('ok.jpg'));
  await Promise.all([...running, cancelled, failure, next]);
  await flush();
  assert.deepEqual(queue.getProgress(), { total: 5, completed: 5, failed: 1, pending: 0 });
  assert.equal(await queue.request('broken', true, async () => 'retry.jpg'), 'retry.jpg');
  await flush();
  assert.deepEqual(queue.getProgress(), { total: 1, completed: 1, failed: 0, pending: 0 });
});

test('visible thumbnails overtake prefetch and queued cards can be promoted',async()=>{
 const queue=new MediaQueue(),finishes:Array<(s:string)=>void>=[],order:string[]=[];
 const running=Array.from({length:3},(_,i)=>queue.request('busy'+i,true,()=>new Promise<string>(resolve=>finishes.push(resolve))));
 const prefetch=queue.request('prefetch',true,async()=>{order.push('prefetch');return 'a';},undefined,1);
 const promoted=queue.request('promoted',true,async()=>{order.push('promoted');return 'b';},undefined,1);
 const visible=queue.request('visible',true,async()=>{order.push('visible');return 'c';},undefined,0);
 queue.prioritize('promoted',0);await flush();finishes[0]('done');await Promise.all([prefetch,promoted,visible]);assert.deepEqual(order,['visible','promoted','prefetch']);finishes.slice(1).forEach(f=>f('done'));await Promise.all(running);
});

test('warms 700 off-screen thumbnails with one background worker and counts failures without stopping',async()=>{
 const queue=new MediaQueue();let active=0,maxActive=0,calls=0;
 const work=Array.from({length:700},(_,i)=>queue.request(`background-${i}`,true,async()=>{
  calls++;active++;maxActive=Math.max(maxActive,active);await Promise.resolve();active--;
  if(i===350)throw Error('Missing original');return `cached-${i}.jpg`;
 },undefined,2));
 assert.equal(queue.getProgress().total,700);
 await Promise.allSettled(work);await flush();
 assert.equal(calls,700);assert.equal(maxActive,1);
 assert.deepEqual(queue.getProgress(),{total:700,completed:700,failed:1,pending:0});
});

test('background warming reserves slots for visible thumbnails and previews',async()=>{
 const queue=new MediaQueue(),order:string[]=[],finishes=new Map<string,(s:string)=>void>();
 const run=(name:string)=>()=>new Promise<string>(resolve=>{order.push(name);finishes.set(name,resolve);});
 const background=queue.request('background',true,run('background'),undefined,2);
 const later=queue.request('later',true,run('later'),undefined,2);await flush();
 assert.deepEqual(order,['background']);
 const visible=queue.request('visible',true,run('visible'),undefined,0);
 const preview=queue.request('preview',false,run('preview'));await flush();
 assert.deepEqual(order,['background','visible','preview']);
 finishes.get('visible')!('ok');finishes.get('preview')!('ok');await Promise.all([visible,preview]);await flush();
 assert(!order.includes('later'));
 finishes.get('background')!('ok');await background;await flush();
 assert.equal(order.at(-1),'later');finishes.get('later')!('ok');await later;
});

test('unmounted foreground cards fall back to background priority and leaving a collection drops queued work',async()=>{
 const queue=new MediaQueue(),finishes:Array<(s:string)=>void>=[];let offscreenCalls=0;
 const backgroundAbort=new AbortController(),cardAbort=new AbortController();
 const background=queue.request('busy-background',true,()=>new Promise<string>(r=>finishes.push(r)),undefined,2);await flush();
 const foreground=Array.from({length:2},(_,i)=>queue.request(`visible-${i}`,true,()=>new Promise<string>(r=>finishes.push(r)),undefined,0));await flush();
 const offscreen=queue.request('offscreen',true,async()=>{offscreenCalls++;return 'cached';},backgroundAbort.signal,2);
 const stopped=assert.rejects(offscreen,/Cancelled/);
 const card=queue.request('offscreen',true,async()=>{throw Error('Duplicate decode');},cardAbort.signal,0);
 const cancelled=assert.rejects(card,/Cancelled/);cardAbort.abort();
 finishes.slice(1).forEach(r=>r('ok'));await Promise.all(foreground);await flush();
 assert.equal(offscreenCalls,0); // Still only one background worker, despite prior promotion.
 backgroundAbort.abort();await stopped;await cancelled;await flush();
 assert.equal(queue.getProgress().pending,1);
 finishes[0]('ok');await background;await flush();assert.equal(offscreenCalls,0);
 assert.deepEqual(queue.getProgress(),{total:3,completed:3,failed:0,pending:0});
});
