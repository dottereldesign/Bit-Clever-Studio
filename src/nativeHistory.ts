import type {Library} from './model';
type NativeEntry=NonNullable<Library['undo_entries']>[number];
// Feed only snapshots accepted by the refresh coordinator. The first snapshot
// (or a different library) is a baseline, not a new edit in this session.
export function createNativeHistoryTracker(){
 let root:string|undefined;
 let known=new Set<number>();
 return (snapshot:Pick<Library,'root'|'undo_entries'>):NativeEntry[]=>{
  const sameLibrary=root!==undefined&&root===snapshot.root;
  const next=new Set<number>(),added:NativeEntry[]=[];
  for(const entry of snapshot.undo_entries??[]){
   if(sameLibrary&&!known.has(entry.id)&&!next.has(entry.id))added.push(entry);
   next.add(entry.id);
  }
  root=snapshot.root;known=next;
  return added;
 };
}
