import {createPortal} from 'react-dom';
import useHomeShortcutDrag from './useHomeShortcutDrag';
import {useState,useEffect,useRef,type ReactNode} from 'react';
import {ArrowRight,Folder,FolderOpen,Plus,Star,Copy,Check} from 'lucide-react';
import SidebarSection from './SidebarSection';
import {AppIcon,type IconValue} from './AppIcon';
import './codeLibrary.css';
import './codeButtonStyles.css';
import {exportCodeButton} from './codeExport';

import {useHistoryState,HistoryButtons} from './useAppHistory';
import {codeButtons as buttons,defaultCodePrefs,normalizeCodePrefs,codeDescendants,codeFolderItems,codeFolderPath,type CodeFolder} from './codeLibraryData';
import FolderRename from './FolderRename';
import {StudioDialog} from './Shortcuts';
const useCodePrefs=()=>useHistoryState('code-library',()=>{try{return normalizeCodePrefs(JSON.parse(localStorage.getItem('code-library')??'null'));}catch{return defaultCodePrefs();}},'Code library');
export function CodeNavigation({name,styleId,icon,active,page,open,reorder}:{name:string;styleId?:number;icon?:IconValue;active:boolean;page?:string;open:(page:string)=>void;reorder?:{up?:()=>void;down?:()=>void}}){
 const [prefs,setPrefs]=useCodePrefs(),[menu,setMenu]=useState<{id:string;x:number;y:number}|null>(null),[creating,setCreating]=useState<string|null>(null),[nameValue,setName]=useState(''),[error,setError]=useState('');
 const drag=useHomeShortcutDrag(()=>{},()=>{},(id,target,after)=>{const from=id.slice(5),to=target.slice(5);setPrefs(previous=>{const source=previous.folders.find(f=>f.id===from),destination=previous.folders.find(f=>f.id===to);if(!source||!destination||source.id==='all'||source.id===destination.id||source.parent!==destination.parent)return previous;const folders=previous.folders.filter(f=>f.id!==from);folders.splice(folders.findIndex(f=>f.id===to)+(after?1:0),0,source);return {...previous,folders};});},false);
 const selected=prefs.folders.some(f=>f.id===page)?page:'all';
 const update=(id:string,patch:Partial<CodeFolder>)=>setPrefs(previous=>({...previous,folders:previous.folders.map(f=>f.id===id?{...f,...patch}:f)}));
 const create=(parent:string)=>{setMenu(null);setCreating(parent);setName('');setError('');};
 const row=(folder:CodeFolder):ReactNode=><div key={folder.id}><button className={`nav-item source-item ${folder.id==='all'?'source-parent':''} ${active&&selected===folder.id?'source-active':''}`} {...drag.props(`code:${folder.id}`,folder.name)} data-folder-order={folder.id==='all'?undefined:`code:${folder.parent}`} data-code-folder={folder.id} title={codeFolderPath(prefs.folders,folder)} aria-current={active&&selected===folder.id?'page':undefined} onClick={()=>open(folder.id)} onContextMenu={e=>{e.preventDefault();e.stopPropagation();setMenu({id:folder.id,x:e.clientX,y:e.clientY});}}>{folder.starred&&<Star className="folder-star" size={11} fill="currentColor" aria-label="Starred folder"/>}<i className="folder-row-icon" style={{color:folder.color}}><AppIcon value={folder.icon} fallback={folder.id==='all'?<FolderOpen size={14}/>:<Folder size={14}/>}/></i><span>{folder.name}</span><small>{codeFolderItems(prefs,folder.id).length}</small></button>{prefs.folders.some(f=>f.parent===folder.id)&&<div className="code-folder-children">{prefs.folders.filter(f=>f.parent===folder.id).map(row)}</div>}</div>;
 const current=menu&&prefs.folders.find(f=>f.id===menu.id);
 return <><SidebarSection name={name} identity="code" styleId={styleId} icon={icon&&<AppIcon value={icon}/>} reorder={reorder} action={<button className="icon-button" title="New Code folder" aria-label="New Code folder" onClick={()=>create('all')}><Plus size={15}/></button>}><nav aria-label="Code" onClickCapture={drag.captureClick}>{prefs.folders.filter(f=>f.id==='all').map(row)}</nav></SidebarSection>
 {drag.ghost&&createPortal(<div className="home-shortcut-ghost" style={{left:drag.ghost.x+14,top:drag.ghost.y+14}}>{drag.ghost.label}</div>,document.body)}
 {current&&menu&&<FolderRename key={current.id} target={{name:current.name,x:menu.x,y:menu.y}} close={()=>setMenu(null)} canRename={current.id!=='all'} newFolder={()=>create(current.id)} save={name=>{if(prefs.folders.some(f=>f.id!==current.id&&f.parent===current.parent&&f.name.toLowerCase()===name.toLowerCase()))throw Error('A folder with that name already exists.');update(current.id,{name});}} appearance={{...current,setColor:color=>update(current.id,{color}),setIcon:icon=>update(current.id,{icon}),toggleStar:()=>update(current.id,{starred:!current.starred})}} removalDescription="This removes the folder and its subfolders. The live components stay in All Components and their other folders. You can undo this change." remove={current.id==='all'?undefined:async()=>{const removed=codeDescendants(prefs.folders,current.id);setPrefs(previous=>({...previous,folders:previous.folders.filter(f=>!removed.has(f.id))}));if(page&&removed.has(page))open('all');}}/>}
 {creating&&<StudioDialog title="New folder" close={()=>setCreating(null)}><form className="code-folder-form" onSubmit={e=>{e.preventDefault();const name=nameValue.trim();if(!name||/[\\/\x00-\x1f]/.test(name)){setError('Enter a name without slashes or control characters.');return;}if(prefs.folders.some(f=>f.parent===creating&&f.name.toLowerCase()===name.toLowerCase())){setError('A folder with that name already exists.');return;}const id=crypto.randomUUID();setPrefs(previous=>({...previous,folders:[...previous.folders,{id,name,parent:creating,items:[],columns:2}]}));setCreating(null);open(id);}}><label>Folder name<input autoFocus aria-label="Code folder name" placeholder="e.g. Favourites" value={nameValue} onChange={e=>setName(e.target.value)} required maxLength={100}/></label>{error&&<p role="alert">{error}</p>}<div className="code-dialog-actions"><button type="button" onClick={()=>setCreating(null)}>Cancel</button><button type="submit" className="code-folder-create">Create folder</button></div></form></StudioDialog>}
 </>;
}
export default function CodeLibrary({page='buttons'}:{page?:string}){
 const [width,setWidth]=useState(1000),[copying,setCopying]=useState<string|null>(null),[copied,setCopied]=useState<string|null>(null),[copyError,setCopyError]=useState<{id:string;message:string}|null>(null);
 const copyTimer=useRef<ReturnType<typeof setTimeout>|undefined>(undefined);
 useEffect(()=>()=>clearTimeout(copyTimer.current),[]);
 const copyCode=async(id:string,control:HTMLButtonElement)=>{setCopying(id);setCopyError(null);clearTimeout(copyTimer.current);setCopied(null);try{const preview=control.closest('article')?.querySelector<HTMLButtonElement>('.code-demo');if(!preview)throw Error('Component preview is unavailable.');await navigator.clipboard.writeText(exportCodeButton(id,preview));setCopied(id);copyTimer.current=setTimeout(()=>setCopied(null),2000);}catch(error){setCopyError({id,message:`Could not copy code. ${String(error)}`});}finally{setCopying(null);}};
 const [prefs,setPrefs]=useCodePrefs(),grid=useRef<HTMLDivElement>(null);
 const folder=prefs.folders.find(f=>f.id===page)??prefs.folders.find(f=>f.id==='all')!;
 const ids=codeFolderItems(prefs,folder.id),visible=buttons.filter(b=>ids.includes(b.id));
 useEffect(()=>{const node=grid.current;if(!node)return;const observer=new ResizeObserver(entries=>setWidth(entries[0].contentRect.width));observer.observe(node);return()=>observer.disconnect();},[]);
 const columns=Math.min(folder.columns,Math.max(1,Math.floor((width+24)/300)));

 return <section className="code-library" aria-labelledby="code-library-title">
  <header className="code-library-heading"><div><h1 id="code-library-title">{folder.name}<span>{String(visible.length).padStart(2,'0')}</span></h1></div></header>
  <div className="code-library-toolbar"><label>Columns <input aria-label="Code columns" type="range" min="1" max="4" value={folder.columns} onChange={e=>{const columns=Number(e.target.value);setPrefs(previous=>({...previous,folders:previous.folders.map(f=>f.id===folder.id?{...f,columns}:f)}));}}/><output>{folder.columns}</output>{columns<folder.columns&&<small>{columns} fit this window</small>}</label><HistoryButtons/></div>
  <div className="code-library-grid" ref={grid} style={{gridTemplateColumns:`repeat(${columns},minmax(0,1fr))`}}>{visible.map(button=><article className={`code-specimen code-specimen-${button.id}`} key={button.id} aria-labelledby={`code-title-${button.id}`}>
   <div className="code-specimen-stage"><button type="button" className={`code-demo code-demo-${button.id}`}><span className="code-demo-label">Placeholder</span><span className="code-demo-arrow" aria-hidden="true"><ArrowRight size={18} strokeWidth={1.75}/>{button.id==='obsidian'&&<ArrowRight className="code-arrow-incoming" size={18} strokeWidth={1.75}/>}</span></button></div>
   <footer><h2 id={`code-title-${button.id}`}>{button.name}</h2><button className="code-copy" title={copied===button.id?"Copied!":`Copy ${button.name} code`} aria-label={copied===button.id?`Copied ${button.name} code`:`Copy ${button.name} code`} disabled={copying!==null} onClick={e=>void copyCode(button.id,e.currentTarget)}>{copied===button.id?<Check size={16}/>:<Copy size={16}/>}</button></footer>
   {copyError?.id===button.id&&<p className="code-copy-error" role="alert">{copyError.message}</p>}
  </article>)}</div>
  {!visible.length&&<div className="code-library-empty"><FolderOpen size={28}/><h2>This folder is ready</h2><p>No components in this folder yet.</p></div>}
 </section>;
}
