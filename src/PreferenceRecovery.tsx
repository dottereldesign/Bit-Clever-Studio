import {useState} from 'react';
import {recoverPreferences} from './backupPreferences';
import {previousPreferences,recoveryKey} from './preferenceStorage';

export default function PreferenceRecovery(){
  const [busy,setBusy]=useState(false),[error,setError]=useState('');
  const repaired=localStorage.getItem(recoveryKey(localStorage));
  const run=async(restore:boolean)=>{setBusy(true);setError('');try{await recoverPreferences(restore);window.location.reload();}catch(e){setError(String(e));setBusy(false);}};
  return <section aria-label="Preference recovery">
    <h3>Preference recovery</h3>
    {repaired&&<p role="status">Some saved preferences were repaired. The original values are retained locally.</p>}
    <p>Reset appearance, layouts and shortcuts to their defaults. Projects, files, drafts and library content are kept. You can restore the preferences saved before the last reset.</p>
    <button disabled={busy} onClick={()=>void run(false)}>Reset preferences</button>{' '}
    {previousPreferences()&&<button disabled={busy} onClick={()=>void run(true)}>Restore previous preferences</button>}
    {repaired&&<button disabled={busy} onClick={()=>{const url=URL.createObjectURL(new Blob([repaired],{type:'application/json'}));const link=document.createElement('a');link.href=url;link.download='preference-recovery.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}}>Download original preferences</button>}
    {error&&<p role="alert">Preferences were not changed: {error}</p>}
  </section>;
}
