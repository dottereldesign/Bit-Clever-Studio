import { useEffect, useRef, useState, useSyncExternalStore, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, Folder, Image, Palette, X } from "lucide-react";
import { mediaQueue } from "./api";
import "./thumbnailProgress.css";

export const loadingStyles = [
  ["pill", "Soft pill", "A softer version of the original."],
  ["line", "Quiet line", "Lightweight and tucked beside the title."],
  ["ring", "Progress ring", "A small circular meter with a clear number."],
  ["segments", "Segments", "Ten steps, easy to read at a glance."],
  ["underline", "Header underline", "A low-profile line along the lower edge."],
  ["capsule", "Accent capsule", "A stronger colour accent for busy headers."],
  ["tiles", "Thumbnail tiles", "A tiny image grid that fills as work completes."],
  ["dots", "Dot trail", "Small, understated progress dots."],
  ["number", "Number first", "The percentage takes the lead."],
  ["orbit", "Orbit", "A moving ring with a steady percentage."],
] as const;
export type LoadingStyle = typeof loadingStyles[number][0];

export function LoadingIndicator({style, percent, label="Thumbnails", detail, busy=true}:{style:LoadingStyle;percent:number;label?:string;detail?:string;busy?:boolean}) {
  const value=Math.max(0,Math.min(100,percent));
  return <div className={`tl-indicator tl-${style}${busy?" is-busy":""}`} style={{"--tl-progress":`${value}%`} as CSSProperties} role="progressbar" aria-label="Thumbnail loading" aria-valuemin={0} aria-valuemax={100} aria-valuenow={value} aria-valuetext={detail??`${label}: ${value}%`} title={detail}>
    <span aria-hidden="true" className="tl-visual">
      {(style==="ring"||style==="orbit")&&<span className="tl-ring"><span>{value}<small>%</small></span></span>}
      {style==="tiles"&&<span className="tl-tiles">{Array.from({length:4},(_,i)=><i key={i} className={value>i*25?"filled":""}><Image size={8}/></i>)}</span>}
      <span className="tl-copy"><span className="tl-label">{label}</span><span className="tl-percent">{value}%</span></span>
      {(style==="segments"||style==="dots")?<span className="tl-steps">{Array.from({length:10},(_,i)=><i key={i} className={value>i*10?"filled":""}/>)}</span>:!["ring","orbit","tiles"].includes(style)&&<span className="tl-track"><i/></span>}
    </span>
  </div>;
}

function StylePicker({selected,choose,close,title,percent,setPercent}:{selected:LoadingStyle;choose:(style:LoadingStyle)=>void;close:()=>void;title:string;percent:number;setPercent:(value:number)=>void}) {
  const dialog=useRef<HTMLDialogElement>(null);
  useEffect(()=>{const node=dialog.current;const opener=document.activeElement as HTMLElement|null;node?.showModal();return()=>{node?.close();if(opener?.isConnected)opener.focus({preventScroll:true});};},[]);
  return createPortal(<dialog ref={dialog} className="tl-picker" aria-labelledby="tl-picker-title" onCancel={event=>{event.preventDefault();close();}} onClick={event=>{if(event.target===event.currentTarget)close();}} onKeyDown={event=>{
      event.stopPropagation();
      if(event.key!=="Tab")return;
      const nodes=[...event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled)')];
      const first=nodes[0],last=nodes.at(-1);
      if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}
      else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first?.focus();}
    }}>
    <div className="tl-picker-inner">
      <div className="tl-picker-heading"><div><span className="tl-eyebrow">HEADER DETAILS</span><h2 id="tl-picker-title">Thumbnail loading</h2><p>Ten ways to keep progress in view. Pick one to try in your header.</p></div><button type="button" className="tl-close" aria-label="Close loading styles" onClick={close}><X size={18}/></button></div>
      <div className="tl-demo-control"><label htmlFor="tl-sample">Sample progress <strong>{percent}%</strong></label><input id="tl-sample" type="range" min="0" max="100" value={percent} onChange={event=>setPercent(Number(event.target.value))}/></div>
      <div className="tl-options">{loadingStyles.map(([style,name,description],index)=><button type="button" className={`tl-option${selected===style?" selected":""}`} key={style} aria-pressed={selected===style} onClick={()=>choose(style)}>
        <span className="tl-sample-header"><span className="tl-sample-title"><Folder size={14}/><span>{title}</span><ChevronDown size={11}/></span><LoadingIndicator style={style} percent={percent} busy={percent<100} label={percent===100?"Ready":"Thumbnails"}/></span>
        <span className="tl-option-caption"><span className="tl-option-number">{String(index+1).padStart(2,"0")}</span><span><strong>{name}</strong><small>{description}</small></span><span className="tl-selected">{selected===style&&<Check size={15}/>}</span></span>
      </button>)}</div>
      <div className="tl-picker-footer"><span>Try any style, then close to see it in place. Choices last until reload.</span><button type="button" onClick={close}>View in header</button></div>
    </div>
  </dialog>,document.body);
}

export default function ThumbnailProgress({title="Design Inspo"}:{title?:string}) {
  const progress = useSyncExternalStore(mediaQueue.subscribe, mediaQueue.getProgress);
  const [visible, setVisible] = useState(false);
  const [style,setStyle]=useState<LoadingStyle>("pill");
  const [open,setOpen]=useState(false);
  const [preview,setPreview]=useState(false);
  const [sample,setSample]=useState(30);
  const busy = progress.pending > 0;
  useEffect(() => {
    const timer = window.setTimeout(() => setVisible(busy), busy ? 180 : 1200);
    return () => window.clearTimeout(timer);
  }, [busy]);
  const actual=visible&&progress.total>0;
  const percent=actual?Math.floor(progress.completed/progress.total*100):sample;
  const label=actual?(busy?"Thumbnails":progress.failed?"Checked":"Ready"):sample===100?"Ready":"Thumbnails";
  const detail=actual?`${progress.completed} of ${progress.total} requested thumbnails processed${progress.failed?` \u00b7 ${progress.failed} unavailable; use Retry on a card`:""}`:`Style preview: ${sample}%, not live thumbnail loading`;
  return <div className="tl-header-control">
    {(actual||preview)&&<LoadingIndicator style={style} percent={percent} label={label} detail={detail} busy={actual?busy:sample<100}/>}
    {preview&&!actual&&<button type="button" className="tl-preview-stop" onClick={()=>setPreview(false)} title="Stop sample preview">Preview <X size={10}/></button>}
    <button type="button" className="tl-style-trigger" aria-label="Thumbnail loading styles" title="Compare 10 thumbnail loading styles" aria-haspopup="dialog" onClick={()=>setOpen(true)}><Palette size={14}/><span>Loading styles</span></button>
    {open&&<StylePicker percent={sample} setPercent={setSample} title={title} selected={style} choose={value=>{setStyle(value);setPreview(true);}} close={()=>setOpen(false)}/>}
  </div>;
}
