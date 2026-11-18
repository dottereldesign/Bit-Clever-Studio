import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {Eye,X} from 'lucide-react';
import {sidebarSections} from './sidebarVisibilityData';
import {useSidebarVisibility} from './useSidebarVisibility';
import './sidebarVisibility.css';

export default function SidebarVisibility(){
  const [hidden,setHidden]=useSidebarVisibility();
  const [open,setOpen]=useState(false);
  const trigger=useRef<HTMLButtonElement>(null),dialog=useRef<HTMLDialogElement>(null);
  const close=()=>{setOpen(false);requestAnimationFrame(()=>trigger.current?.focus());};
  useLayoutEffect(()=>{
    if(!open)return;
    const place=()=>{
      const menu=dialog.current,sidebar=trigger.current?.closest('.sidebar'),button=trigger.current;
      if(!menu||!sidebar||!button)return;
      const edge=sidebar.getBoundingClientRect(),anchor=button.getBoundingClientRect();
      menu.style.left=`${Math.max(8,Math.min(edge.right,window.innerWidth-menu.offsetWidth-8))}px`;
      menu.style.top=`${Math.max(8,Math.min(anchor.top,window.innerHeight-menu.offsetHeight-8))}px`;
    };
    dialog.current?.showModal();place();
    const observer=new ResizeObserver(place);if(trigger.current)observer.observe(trigger.current);
    window.addEventListener('resize',place);
    return()=>{observer.disconnect();window.removeEventListener('resize',place);dialog.current?.close();};
  },[open]);
  useEffect(()=>{if(open)dialog.current?.querySelector<HTMLInputElement>('input')?.focus();},[open]);
  return <><button ref={trigger} type="button" title="Show or hide sidebar sections" aria-label="Sidebar visibility" aria-haspopup="dialog" aria-expanded={open} onClick={()=>setOpen(!open)}><Eye size={16}/></button>
    {open&&createPortal(<dialog ref={dialog} className="sidebar-visibility-menu" aria-labelledby="sidebar-visibility-title" onCancel={event=>{event.preventDefault();close();}} onKeyDown={event=>event.stopPropagation()} onClick={event=>{if(event.target===event.currentTarget){const r=event.currentTarget.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)close();}}}>
      <header><h2 id="sidebar-visibility-title">Sidebar sections</h2><button type="button" aria-label="Close sidebar visibility" onClick={close}><X size={16}/></button></header>
      <p>Choose what you see in your sidebar.</p>
      <fieldset><legend className="sidebar-visibility-sr">Visible sections</legend>{sidebarSections.map(([id,label])=><label key={id}><input type="checkbox" checked={!hidden.includes(id)} onChange={event=>{const checked=event.target.checked;setHidden(previous=>checked?previous.filter(name=>name!==id):[...previous,id]);}}/><span>{label}</span></label>)}</fieldset>
      <footer><button type="button" disabled={!hidden.length} onClick={()=>setHidden([])}>Show all</button><span>{sidebarSections.length-hidden.length} of {sidebarSections.length} visible</span></footer>
    </dialog>,document.body)}
  </>;
}
