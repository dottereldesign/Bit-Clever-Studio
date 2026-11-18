import {useLayoutEffect,useRef,type KeyboardEvent} from 'react';
// Div dialogs need both a keyboard boundary and focus restoration. Native file
// pickers and nested crop dialogs keep their own focus handling.
export function useModalFocus(close:()=>void,busy=false,suspended=false){
 const ref=useRef<HTMLDivElement>(null),opener=useRef(document.activeElement as HTMLElement|null);
 useLayoutEffect(()=>{
  const node=ref.current;
  if(node&&!node.contains(document.activeElement)) (node.querySelector<HTMLElement>('input:not(:disabled),textarea:not(:disabled),button:not(:disabled)')??node).focus();
  return()=>{requestAnimationFrame(()=>{if(node?.isConnected)return;const target=opener.current?.isConnected?opener.current:document.querySelector<HTMLElement>('[aria-label="Settings"]');target?.focus({preventScroll:true});});};
 },[]);
 useLayoutEffect(()=>{
  const node=ref.current;
  if(!suspended&&node&&!node.contains(document.activeElement))node.focus({preventScroll:true});
 },[busy,suspended]);
 const onKeyDownCapture=(event:KeyboardEvent<HTMLDivElement>)=>{
  if(suspended)return;
  if(event.key==='Escape'){event.preventDefault();event.stopPropagation();if(!busy)close();return;}
  if(event.key!=='Tab')return;
  const root=event.currentTarget;
  const nodes=[...root.querySelectorAll<HTMLElement>('button,input,textarea,select,a[href],[tabindex]')].filter(n=>n.tabIndex>=0&&!n.matches(':disabled')&&!n.closest('[inert]')&&n.getClientRects().length);
  const first=nodes[0],last=nodes.at(-1),active=document.activeElement;
  if(!first){event.preventDefault();root.focus();}
  else if(event.shiftKey&&(active===first||!nodes.includes(active as HTMLElement))){event.preventDefault();last?.focus();}
  else if(!event.shiftKey&&(active===last||!nodes.includes(active as HTMLElement))){event.preventDefault();first.focus();}
 };
 return {ref,tabIndex:-1,onKeyDownCapture};
}
