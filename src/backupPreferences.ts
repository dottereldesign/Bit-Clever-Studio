import {record} from './diagnostics';
import {invoke,isTauri} from "./diagnostics";
import {appHistory} from "./appHistory";
import {isRecord,isPreference} from './preferenceSchema';
import {applyPreferenceValues,readPreferenceValues,repairPreferences,recoverStoredPreferences,retainOriginal} from './preferenceStorage';
export const collectBackupPreferences=()=>readPreferenceValues();
export const applyBackupPreferences=(values:unknown)=>applyPreferenceValues(values);

export async function loadLibraryPreferences(){
 if(!isTauri()){repairPreferences();return;}
 const {root}=await invoke<{root:string}>('library_overview');
 if(localStorage.getItem('active-library-root')!==root){localStorage.removeItem('last-workspace-view');localStorage.setItem('active-library-root',root);}
 const values=await invoke<unknown>('library_preferences');
 if(values!==null){
  if(!isRecord(values)){retainOriginal(localStorage,'library-preferences',JSON.stringify(values));throw Error('Saved preferences are invalid. Use Reset preferences to recover.');}
  for(const key of Object.keys(localStorage))if(isPreference(key))localStorage.removeItem(key);
  applyBackupPreferences(values);
 }
 repairPreferences();
}
let pending:Promise<void>|null=null,dirty=false,last='';
export async function flushLibraryPreferences(){
 if(!isTauri())return;
 dirty=true;if(pending)return pending;
 pending=(async()=>{try{while(dirty){dirty=false;const preferences=collectBackupPreferences(),text=JSON.stringify(preferences);if(text!==last){await invoke('save_library_preferences',{preferences});record('settings.saved','success',0,{count:Object.keys(preferences).length});last=text;}}}finally{pending=null;}})();
 return pending;
}
export function trackLibraryPreferences(report:(e:unknown)=>void){return appHistory.subscribe(()=>{void flushLibraryPreferences().catch(report);});}

// Persist first; a failed native save restores the local values and keeps the view open.
export async function recoverPreferences(restore:boolean){
 if(pending)await pending;
 const recovery=recoverStoredPreferences(restore,async preferences=>{
  if(isTauri())await invoke('save_library_preferences',{preferences});
  last=JSON.stringify(preferences);
 });
 pending=recovery;
 try{await recovery;}finally{pending=null;}
}
