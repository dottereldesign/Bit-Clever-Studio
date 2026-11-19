import {normalizeCodePrefs} from './codeLibraryData';
import {folderSorts} from './folderViews';
import {normalizeActiveColours} from './activeColours';
import {defaultAppearance} from './defaultAppearance';

export const preferenceKeys=['code-library','active-colours','sidebar-hidden-sections','app-theme','app-colour-sources','personal-studio-default','settings-appearance','customise-tab-order','analytics-appearance','home-overview','tab-appearance','cta-appearance','app-branding','clients-startup','clients-last-seen','sidebar-section-order','sidebar-active-green','sidebar-emerald-default-applied','research-view','onboarding','sidebar-section-colours','studio-layouts','project-file-layouts','folder-view-settings','selection-style','colour-history','collection-shortcuts','site-shortcuts','tile-size','sidebar-width','sidebar-section-style','home-style','home-appearance','reduce-motion','sidebar-layout','app-icons','folder-labels','folder-appearance','sidebar-folder-order','home-plan','home-tiles'];
export const isPreference=(key:string)=>preferenceKeys.includes(key)||key.startsWith('sidebar-open:');
export const isStoredPreference=(key:string)=>isPreference(key)||key==='recent-work'||/^(project-scroll|project-files-location|client-view):/.test(key);
export const isRecord=(value:unknown):value is Record<string,unknown>=>value!==null&&typeof value==='object'&&!Array.isArray(value);
const string=(v:unknown)=>typeof v==='string'?v:undefined;
const boolean=(v:unknown)=>typeof v==='boolean'?v:undefined;
const number=(v:unknown)=>typeof v==='number'&&Number.isFinite(v)?v:undefined;
const strings=(v:unknown)=>Array.isArray(v)?v.filter(x=>typeof x==='string'):undefined;
const oneOf=(v:unknown,choices:string[])=>typeof v==='string'&&choices.includes(v)?v:undefined;
const map=(v:unknown,clean:(v:unknown)=>unknown)=>isRecord(v)?Object.fromEntries(Object.entries(v).filter(([k])=>!['__proto__','constructor','prototype'].includes(k)).flatMap(([k,x])=>{const value=clean(x);return value===undefined?[]:[[k,value]];})):undefined;
// Preserve compatible fields and fill missing/invalid fields from the schema.
function shaped(value:unknown,template:unknown):unknown {
  if(Array.isArray(template))return strings(value)??template;
  if(isRecord(template))return Object.fromEntries(Object.entries(template).map(([k,v])=>[k,shaped(isRecord(value)?value[k]:undefined,v)]));
  return typeof value===typeof template&&(typeof value!=='number'||Number.isFinite(value))?value:template;
}
function shortcuts(value:unknown) {
  if(!Array.isArray(value))return undefined;
  const seen=new Set<string>();
  return value.flatMap(v=>{
    if(!isRecord(v)||typeof v.key!=='string'||typeof v.name!=='string'||seen.has(v.key))return [];
    seen.add(v.key);
    const target=isRecord(v.target)&&oneOf(v.target.kind,['view','project','category','website'])&&typeof v.target.value==='string'?v.target:undefined;
    return [{key:v.key,name:v.name,...(typeof v.url==='string'?{url:v.url}:{}),...(target?{target:{kind:target.kind,value:target.value}}:{}),...(typeof v.pinned==='boolean'?{pinned:v.pinned}:{}),...(typeof v.hidden==='boolean'?{hidden:v.hidden}:{}),...(oneOf(v.layout,['auto','masonry','cards','list','table'])?{layout:v.layout}:{}),...(oneOf(v.content,['all','image','video','link'])?{content:v.content}:{})}];
  });
}
function homeTiles(value:unknown) {
  if(!Array.isArray(value))return undefined;
  const seen=new Set<string>();
  return value.filter((v):v is Record<string,unknown>=>isRecord(v)&&typeof v.id==='string'&&!seen.has(v.id)&&!!seen.add(v.id)).slice(0,100).map(v=>{
    const span=number(v.span),cover=v.cover;
    const validCover=isRecord(cover)&&typeof cover.path==='string'&&(number(cover.width)??0)>0&&(number(cover.height)??0)>0;
    return {
      id:v.id,
      ...(span===undefined?{}:{span:Math.max(2,Math.min(12,span))}),
      height:Math.max(128,Math.min(512,number(v.height)??224)),
      radius:Math.max(0,Math.min(48,number(v.radius)??18)),
      autoImageSize:v.autoImageSize===true,
      ...(validCover?{cover:{path:cover.path,width:cover.width,height:cover.height}}:{}),
    };
  });
}
export function normalizePreference(key:string,value:unknown):unknown {
  if(!isStoredPreference(key))return value;
  if(key.startsWith('project-scroll:'))return map(value,number);
  if(key.startsWith('project-files-location:'))return value===null?null:number(value)??oneOf(value,['unfiled']);
  if(key.startsWith('client-view:'))return isRecord(value)?shaped(value,{selected:'',filter:'all',search:'',contacts:false}):undefined;
  if(key==='recent-work')return isRecord(value)&&typeof value.root==='string'?{root:value.root,items:Array.isArray(value.items)?value.items.filter(v=>isRecord(v)&&oneOf(v.kind,['studio','project','item'])&&typeof v.id==='string'):[]}:undefined;
  if(key==='home-plan'){
    if(value===null)return null;
    if(!isRecord(value)||!Number.isInteger(value.count)||Number(value.count)<0)return undefined;
    return {count:value.count,splits:map(value.splits,v=>isRecord(v)&&Number.isInteger(v.split)&&oneOf(v.axis,['x','y'])&&number(v.ratio)!==undefined?{split:v.split,axis:v.axis,ratio:Math.max(0,Math.min(1,v.ratio as number))}:undefined)??{}};
  }
  if(key==='personal-studio-default')return map(value,v=>v)===undefined?undefined:Object.fromEntries(Object.entries(value as Record<string,unknown>).filter(([k])=>isPreference(k)&&k!=='personal-studio-default').flatMap(([k,v])=>{const next=v===null?null:normalizePreference(k,v);return next===undefined?[]:[[k,next]];}));
  if(key==='collection-shortcuts'||key==='site-shortcuts')return shortcuts(value);
  if(['sidebar-hidden-sections','customise-tab-order','sidebar-section-order'].includes(key))return strings(value);
  if(key.startsWith('sidebar-open:')||['reduce-motion','sidebar-emerald-default-applied'].includes(key))return boolean(value);
  if(key==='code-library')return normalizeCodePrefs(value);
  if(key==='active-colours')return normalizeActiveColours(value);
  if(key==='sidebar-active-green')return oneOf(value,['soft','sage','mint','lime','olive','fern','eucalyptus','forest','emerald','pine']);
  const ranges:Record<string,[number,number]>={'tile-size':[80,1000],'sidebar-width':[160,600],'selection-style':[1,50],'sidebar-section-style':[1,1000],'home-style':[1,20],'sidebar-layout':[1,20]};
  if(key in ranges){const n=number(value);return n===undefined?undefined:Math.max(ranges[key][0],Math.min(ranges[key][1],n));}
  if(['folder-labels','sidebar-section-colours'].includes(key))return map(value,string);
  if(key==='clients-last-seen')return map(value,number);
  if(key==='sidebar-folder-order')return map(value,strings);
  if(key==='app-colour-sources')return map(value,v=>oneOf(v,['custom','theme']));
  if(key==='studio-layouts')return map(value,v=>oneOf(v,['table','cards','board','list']));
  if(key==='research-view')return map(value,string);
  if(key==='folder-appearance')return map(value,v=>isRecord(v)?Object.fromEntries(Object.entries(v).filter(([k,x])=>(k==='color'&&typeof x==='string')||(k==='starred'&&typeof x==='boolean'))):undefined);
  if(key==='folder-view-settings')return map(value,group=>map(group,v=>isRecord(v)?{...(number(v.size)!==undefined?{size:Math.max(140,Math.min(420,v.size as number))}:{}),...(oneOf(v.sort,folderSorts)?{sort:v.sort}:{})}:undefined));
  if(key==='project-file-layouts'){
    const clean=(v:unknown)=>isRecord(v)?{layout:oneOf(v.layout,['details','tiles','columns'])??'details',columns:(strings(v.columns)??['type','size','modified','rating']).filter(c=>['type','size','modified','rating'].includes(c)),nameWidth:Math.max(120,Math.min(1000,number(v.nameWidth)??360))}:undefined;
    return map(value,v=>isRecord(v)?{...clean(v),...(isRecord(v.defaults)?{defaults:clean(v.defaults)}:{}),...(isRecord(v.folders)?{folders:map(v.folders,clean)}:{})}:undefined);
  }
  if(key==='app-icons')return isRecord(value)?{folder:string(value.folder)??'folder',overrides:map(value.overrides,v=>isRecord(v)&&oneOf(v.kind,['symbol','image','asset','text'])&&typeof v.value==='string'?{kind:v.kind,value:v.value}:undefined)??{}}:undefined;
  if(key==='colour-history')return Array.isArray(value)?value.filter(v=>isRecord(v)&&typeof v.hex==='string'&&/^#[0-9a-f]{6}$/i.test(v.hex)).map(v=>({hex:v.hex,projects:strings(v.projects)??[]})):undefined;
  if(key==='home-tiles')return homeTiles(value);
  const templates:Record<string,unknown>={...defaultAppearance,'app-theme':{theme:'studio',fontMode:'default',body:'original',heading:'original'},'settings-appearance':{layout:'tabs',compact:false,remember:false},onboarding:{enabled:true,leadsSeen:false},'clients-startup':{}};
  if(key in templates)return isRecord(value)?shaped(value,templates[key]):undefined;
  return undefined;
}
