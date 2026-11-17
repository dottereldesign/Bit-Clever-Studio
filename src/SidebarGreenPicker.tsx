import {useEffect,useLayoutEffect,useRef,useState,type CSSProperties} from 'react';
import {createPortal} from 'react-dom';
import {Palette,X} from 'lucide-react';
import {useHistoryState} from './useAppHistory';
import './sidebarGreenPicker.css';

const greens=[
  {id:'soft',name:'Soft green',background:'#e1ebdc',ink:'#35512a'},
  {id:'sage',name:'Sage',background:'#cad8bd',ink:'#304326'},
  {id:'mint',name:'Mint',background:'#d2f0df',ink:'#22503a'},
  {id:'lime',name:'Fresh lime',background:'#dcecaa',ink:'#374719'},
  {id:'olive',name:'Olive',background:'#c5cc9a',ink:'#354018'},
  {id:'fern',name:'Fern',background:'#a8c896',ink:'#253f21'},
  {id:'eucalyptus',name:'Eucalyptus',background:'#9bc8b6',ink:'#173e30'},
  {id:'forest',name:'Forest',background:'#426b43',ink:'#ffffff'},
  {id:'emerald',name:'Emerald',background:'#247855',ink:'#ffffff'},
  {id:'pine',name:'Deep pine',background:'#254c3b',ink:'#ffffff'},
] as const;

export default function SidebarGreenPicker(){
  const [saved,setSaved]=useHistoryState<string>('sidebar-active-green',()=>{try{return JSON.parse(localStorage.getItem('sidebar-active-green')??'"soft"');}catch{return 'soft';}},'Sidebar active green');
  const selected=greens.find(g=>g.id===saved)??greens[0];
  const [open,setOpen]=useState(false);
  const trigger=useRef<HTMLButtonElement>(null),panel=useRef<HTMLDivElement>(null);
  useLayoutEffect(()=>{
    const style=document.documentElement.style;
    style.setProperty('--sidebar-active-bg',selected.background);style.setProperty('--sidebar-active-ink',selected.ink);
    return()=>{style.removeProperty('--sidebar-active-bg');style.removeProperty('--sidebar-active-ink');};
  },[selected]);
  useEffect(()=>{
    if(!open)return;
    panel.current?.querySelector<HTMLInputElement>('input:checked')?.focus();
    const outside=(event:PointerEvent)=>{if(!panel.current?.contains(event.target as Node)&&!trigger.current?.contains(event.target as Node))setOpen(false);};
    const escape=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();event.stopPropagation();setOpen(false);trigger.current?.focus();}};
    document.addEventListener('pointerdown',outside);document.addEventListener('keydown',escape,true);
    return()=>{document.removeEventListener('pointerdown',outside);document.removeEventListener('keydown',escape,true);};
  },[open]);
  return createPortal(<div className="sidebar-green-picker">
    <button ref={trigger} className="sidebar-green-trigger" aria-label="Green variations" aria-expanded={open} aria-haspopup="dialog" onClick={()=>setOpen(!open)}><Palette size={15}/><span>Green variations</span><i style={{background:selected.background}}/></button>
    {open&&<div ref={panel} className="sidebar-green-panel" role="dialog" aria-labelledby="sidebar-green-title" onKeyDown={event=>event.stopPropagation()}>
      <header><h2 id="sidebar-green-title">Active folder colour</h2><button aria-label="Close green variations" onClick={()=>{setOpen(false);trigger.current?.focus();}}><X size={16}/></button></header>
      <p>Select a folder, then try a green. Changes apply live.</p>
      <fieldset><legend>10 green variations</legend><div className="sidebar-green-options">{greens.map((green,index)=><label key={green.id} style={{'--green-background':green.background,'--green-ink':green.ink} as CSSProperties}><input type="radio" name="sidebar-active-green" value={green.id} checked={selected.id===green.id} onChange={()=>setSaved(green.id)}/><span className="sidebar-green-sample"><span>{String(index+1).padStart(2,'0')}</span>Selected folder</span><span className="sidebar-green-name">{green.name}</span></label>)}</div></fieldset>
      <footer><span>{selected.name}</span><span>Saved automatically</span></footer>
    </div>}
  </div>,document.body);
}
