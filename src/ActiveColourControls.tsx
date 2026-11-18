import {useLayoutEffect} from 'react';
import {Folder} from 'lucide-react';
import {useHistoryState} from './useAppHistory';
import {appHistory} from './appHistory';
import {activeGroups,activeParts,readActiveColours,type ActiveGroup} from './activeColours';
import './activeColours.css';
export function useActiveColours(){
 const [colours]=useHistoryState('active-colours',readActiveColours,'Active colours');
 useLayoutEffect(()=>{
  const root=document.documentElement;
  for(const group of activeGroups)for(const part of activeParts){const name=`active-${group}-${part}`,value=colours[group]?.[part];root.toggleAttribute(`data-${name}`,!!value);if(value)root.style.setProperty(`--${name}`,value);else root.style.removeProperty(`--${name}`);}
  return()=>{for(const group of activeGroups)for(const part of activeParts){root.removeAttribute(`data-active-${group}-${part}`);root.style.removeProperty(`--active-${group}-${part}`);}};
 },[colours]);
}
export default function ActiveColourControls({group}:{group:ActiveGroup}){
 const [colours,set]=useHistoryState('active-colours',readActiveColours,'Active colours');
 const values=colours[group]??{};
 const labels={background:'Background',text:'Text',icon:'Icon',count:'Number / count'};
 const title={sidebar:'Sidebar',tabs:'Tabs',buttons:'Buttons'}[group];
 return <section className="active-colour-controls" aria-label={`${title} active colours`}><h3>Active colours</h3>
 <p>{group==='buttons'?'Selected toggle buttons and buttons while pressed.':'Selected '+(group==='sidebar'?'sidebar items and folders.':'tabs.')} Leave a colour on automatic to follow the existing style.</p>
 {activeParts.map(part=><div className="active-colour-row" key={part}><input type="checkbox" aria-label={`Use custom ${title.toLowerCase()} active ${part} colour`} checked={!!values[part]} onChange={e=>{const next={...values};if(e.target.checked)next[part]=part==='background'?'#247855':'#ffffff';else delete next[part];set({...colours,[group]:next});}}/><label>{labels[part]}<input type="color" aria-label={`${title} active ${part} colour`} title={values[part]??'Automatic until customised'} value={values[part]??(part==='background'?'#247855':'#ffffff')} onBlur={()=>appHistory.endGesture()} onChange={e=>{appHistory.beginGesture(`active-${group}-${part}`);set({...colours,[group]:{...values,[part]:e.target.value}});}}/></label><button type="button" disabled={!values[part]} aria-label={`Reset ${title.toLowerCase()} active ${part} colour`} onClick={()=>{const next={...values};delete next[part];set({...colours,[group]:next});}}>Reset</button></div>)}
 <div className="active-colour-preview" style={{background:values.background??'#247855',color:values.text??'#ffffff'}}><Folder size={16} style={{color:values.icon??values.text??'#ffffff'}}/><span>Selected item</span><small style={{color:values.count??values.text??'#ffffff'}}>12</small></div>
 <small>Custom colours keep your exact choices; choose readable contrast. Image and emoji icons keep their original colours.</small>
 </section>;
}
