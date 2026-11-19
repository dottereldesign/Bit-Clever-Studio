import type {IconValue} from './AppIcon';
export const codeButtons=[
 {id:'obsidian',name:'Obsidian'},
] as const;
export type CodeFolder={id:string;name:string;parent:string|null;items:string[];columns:number;color?:string;starred?:boolean;icon?:IconValue};
export type CodePrefs={folders:CodeFolder[]};
const root:CodeFolder={id:'all',name:'All Components',parent:null,items:[],columns:2};
export const defaultCodePrefs=():CodePrefs=>({folders:[{...root},{id:'buttons',name:'Buttons',parent:'all',items:codeButtons.map(b=>b.id),columns:2}]});
export function normalizeCodePrefs(value:unknown):CodePrefs{
 if(!value||typeof value!=='object'||!Array.isArray((value as CodePrefs).folders))return defaultCodePrefs();
 const ids=new Set<string>();const folders:CodeFolder[]=[];
 for(const f of (value as CodePrefs).folders){
  if(!f||typeof f.id!=='string'||!f.id||ids.has(f.id)||typeof f.name!=='string'||!f.name.trim())continue;
  ids.add(f.id);const icon=f.icon&&['symbol','image','asset','text'].includes(f.icon.kind)&&typeof f.icon.value==='string'?f.icon:undefined;
  folders.push({id:f.id,name:f.id==='all'?'All Components':f.name.trim().slice(0,100),parent:f.id==='all'?null:typeof f.parent==='string'?f.parent:'all',items:Array.isArray(f.items)?[...new Set(f.items.filter(id=>codeButtons.some(b=>b.id===id)))]:[],columns:Number.isFinite(f.columns)?Math.max(1,Math.min(4,Math.round(f.columns))):2,color:typeof f.color==='string'&&/^#[0-9a-f]{6}$/i.test(f.color)?f.color:undefined,starred:f.starred===true,icon});
 }
 if(!ids.has('all'))folders.unshift({...root});
 for(const f of folders){if(f.id==='all')continue;const seen=new Set([f.id]);let parent=f.parent;while(parent&&parent!=='all'){if(seen.has(parent)){f.parent='all';break;}seen.add(parent);const next=folders.find(x=>x.id===parent);if(!next){f.parent='all';break;}parent=next.parent;}}
 return {folders};
}
export function codeDescendants(folders:CodeFolder[],id:string):Set<string>{const ids=new Set([id]);let changed=true;while(changed){changed=false;for(const f of folders)if(f.parent&&ids.has(f.parent)&&!ids.has(f.id)){ids.add(f.id);changed=true;}}return ids;}
export function codeFolderItems(prefs:CodePrefs,id:string){if(id==='all')return codeButtons.map(b=>b.id);const ids=codeDescendants(prefs.folders,id);return [...new Set(prefs.folders.filter(f=>ids.has(f.id)).flatMap(f=>f.items))].filter(id=>codeButtons.some(button=>button.id===id));}

export function codeFolderPath(folders:CodeFolder[],folder:CodeFolder):string{const names=[folder.name],seen=new Set([folder.id]);let parent=folder.parent;while(parent&&parent!=='all'&&!seen.has(parent)){seen.add(parent);const f=folders.find(f=>f.id===parent);if(!f)break;names.unshift(f.name);parent=f.parent;}return names.join(' / ');}
