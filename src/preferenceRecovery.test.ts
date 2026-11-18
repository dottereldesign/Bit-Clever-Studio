import {test} from 'node:test';
import assert from 'node:assert/strict';
import {defaultAppearance} from './defaultAppearance';
import {normalizePreference,preferenceKeys} from './preferenceSchema';
import {applyPreferenceValues,readPreferenceValues,repairPreferences,recoveryKey,recoverStoredPreferences,previousPreferences} from './preferenceStorage';
import {AppHistory} from './appHistory';

function storage(seed:Record<string,string>={}):Storage{
  const value={...seed};
  Object.defineProperties(value,{
    getItem:{value:(key:string)=>Object.hasOwn(value,key)?value[key]:null},
    setItem:{value:(key:string,text:string)=>{Object.defineProperty(value,key,{value:String(text),writable:true,enumerable:true,configurable:true});}},
    removeItem:{value:(key:string)=>{delete value[key];}},
  });
  return value as unknown as Storage;
}
test('invalid JSON and parsed null/array/object shapes recover before consumers read them',()=>{
  const s=storage({'folder-appearance':'null','research-view':'[]','project-file-layouts':'{"project:1":{"columns":null}}','site-shortcuts':'[null,{"key":"bad","name":{}},{"key":"good","name":"Keep"}]','app-icons':'{"overrides":{"home":{"kind":"text","value":{}}}}','home-plan':'{'});
  repairPreferences(s);
  assert.equal(s.getItem('folder-appearance'),null);assert.equal(s.getItem('research-view'),null);
  assert.deepEqual(JSON.parse(s.getItem('project-file-layouts')!)['project:1'].columns,['type','size','modified','rating']);
  assert.deepEqual(JSON.parse(s.getItem('site-shortcuts')!),[{key:'good',name:'Keep'}]);
  assert.deepEqual(JSON.parse(s.getItem('app-icons')!).overrides,{});
  const originals=s.getItem(recoveryKey(s));assert.equal(JSON.parse(originals!)['folder-appearance'],'null');
  repairPreferences(s);assert.equal(s.getItem(recoveryKey(s)),originals,'repeated startup preserves the first originals');
});
test('every backed-up preference handles malformed shapes; valid defaults survive unchanged',()=>{
  for(const key of preferenceKeys)for(const value of [null,[],42,'x',true,{bad:null}])assert.doesNotThrow(()=>normalizePreference(key,value));
  for(const [key,value] of Object.entries(defaultAppearance))assert.deepEqual(normalizePreference(key,value),value,key);
  assert.equal(normalizePreference('home-plan',null),null);
  assert.equal(normalizePreference('project-files-location:1',null),null);
});
test('backup restore validates nested values and never exports recovery records or drafts',()=>{
  const s=storage({'draft:private':'"keep"'});
  applyPreferenceValues({'folder-appearance':null,'folder-labels':{good:'Name',bad:[]},'sidebar-open:Projects':{},'draft:private':'overwrite'},s);
  assert.deepEqual(readPreferenceValues(s),{'folder-labels':{good:'Name'}});
  assert.equal(s.getItem('draft:private'),'"keep"');assert.ok(s.getItem(recoveryKey(s)));
  assert.throws(()=>applyPreferenceValues(null,s),/must be an object/);
});
test('reset survives reopen, restores previous preferences, and is scoped to a library',async()=>{
  const s=storage({'active-library-root':'A','sidebar-width':'305','folder-labels':'{"x":"Mine"}','draft:private':'"keep"','video-download-queue:v2:A':'[]'});
  const saved:unknown[]=[];
  await recoverStoredPreferences(false,async values=>{saved.push(values);},s);
  assert.deepEqual(saved,[{}]);assert.ok(previousPreferences(s));assert.equal(s.getItem('draft:private'),'"keep"');
  s.setItem('active-library-root','B');assert.equal(previousPreferences(s),null);
  s.setItem('active-library-root','A');await recoverStoredPreferences(true,async values=>{saved.push(values);},s);
  assert.equal(s.getItem('sidebar-width'),'305');assert.deepEqual(saved[1],{'sidebar-width':305,'folder-labels':{x:'Mine'}});
});
test('failed reset persistence leaves preferences and the earlier reset snapshot intact',async()=>{
  const s=storage({'sidebar-width':'305'});
  await assert.rejects(recoverStoredPreferences(false,async()=>{throw Error('disk full');},s),/disk full/);
  assert.equal(s.getItem('sidebar-width'),'305');assert.equal(previousPreferences(s),null);
});
test('undo and redo cannot place malformed nested preferences into the live cache',async()=>{
  const h=new AppHistory();
  h.commit('Restore preferences',{'project-file-layouts':null},{'project-file-layouts':{'project:1':{columns:null}}});
  assert.deepEqual((h.read('project-file-layouts',()=>({})) as any)['project:1'].columns,['type','size','modified','rating']);
  await h.undo();assert.deepEqual(h.read('project-file-layouts',()=>({})),{});
  await h.redo();assert.deepEqual((h.read('project-file-layouts',()=>({})) as any)['project:1'].columns,['type','size','modified','rating']);
});
