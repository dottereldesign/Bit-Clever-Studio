import {isPreference,isStoredPreference,isRecord,normalizePreference,preferenceKeys} from './preferenceSchema';
import {record} from './diagnostics';

export const recoveryKey=(storage:Storage)=>`preference-recovery:v1:${storage.getItem('active-library-root')??'browser'}`;
export const resetKey=(storage:Storage)=>`preference-reset:v1:${storage.getItem('active-library-root')??'browser'}`;
function canonical(value:unknown):string {
  return JSON.stringify(value,(_,v)=>isRecord(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);
}
export function retainOriginal(storage:Storage,key:string,raw:string) {
  let saved:Record<string,unknown>={};
  try{const parsed=JSON.parse(storage.getItem(recoveryKey(storage))??'{}');if(isRecord(parsed))saved=parsed;}catch{}
  // Preserve the first original, before any normalisation or future save.
  if(!(key in saved)){
    saved[key]=raw;storage.setItem(recoveryKey(storage),JSON.stringify(saved));
    record('settings.invalid_value','warning',0,{field:preferenceKeys.includes(key)?key:'view_preferences',error_code:'invalid_preference'});
  }
}
export function repairStoredPreference(key:string,storage:Storage=localStorage) {
  if(!isStoredPreference(key))return;
  const raw=storage.getItem(key);if(raw===null)return;
  let value:unknown,next:unknown;
  try{value=JSON.parse(raw);next=normalizePreference(key,value);}catch{next=undefined;}
  if(next!==undefined&&canonical(next)===canonical(value))return;
  retainOriginal(storage,key,raw);
  if(next===undefined)storage.removeItem(key);else storage.setItem(key,JSON.stringify(next));
}
export function repairPreferences(storage:Storage=localStorage) {
  for(const key of Object.keys(storage))repairStoredPreference(key,storage);
}
export function readPreferenceValues(storage:Storage=localStorage):Record<string,unknown> {
  repairPreferences(storage);
  return Object.fromEntries(Object.keys(storage).filter(isPreference).map(key=>[key,JSON.parse(storage.getItem(key)!)]));
}
export function applyPreferenceValues(values:unknown,storage:Storage=localStorage) {
  if(!isRecord(values))throw Error('Saved preferences must be an object. Use Reset preferences to recover.');
  for(const [key,value] of Object.entries(values))if(isPreference(key)){
    const next=normalizePreference(key,value);
    if(next===undefined||canonical(next)!==canonical(value))retainOriginal(storage,key,JSON.stringify(value));
    if(next===undefined)storage.removeItem(key);else storage.setItem(key,JSON.stringify(next));
  }
}
export function rawPreferences(storage:Storage=localStorage):Record<string,string> {
  return Object.fromEntries(Object.keys(storage).filter(isStoredPreference).map(key=>[key,storage.getItem(key)!]));
}
export function replaceRawPreferences(values:Record<string,string>,storage:Storage=localStorage) {
  for(const key of Object.keys(storage))if(isStoredPreference(key))storage.removeItem(key);
  for(const [key,raw] of Object.entries(values))if(isStoredPreference(key))storage.setItem(key,raw);
}
export function previousPreferences(storage:Storage=localStorage):Record<string,string>|null {
  try{const value=JSON.parse(storage.getItem(resetKey(storage))??'null');return isRecord(value)&&Object.values(value).every(v=>typeof v==='string')?value as Record<string,string>:null;}catch{return null;}
}
export async function recoverStoredPreferences(restore:boolean,persist:(values:Record<string,unknown>)=>Promise<void>,storage:Storage=localStorage){
  const before=rawPreferences(storage),previous=storage.getItem(resetKey(storage));
  const target=restore?previousPreferences(storage):{};
  if(target===null)throw Error('No previous preferences are available.');
  try{
    if(!restore)storage.setItem(resetKey(storage),JSON.stringify(before));
    replaceRawPreferences(target,storage);repairPreferences(storage);
    await persist(readPreferenceValues(storage));
  }catch(error){replaceRawPreferences(before,storage);if(previous===null)storage.removeItem(resetKey(storage));else storage.setItem(resetKey(storage),previous);throw error;}
}
