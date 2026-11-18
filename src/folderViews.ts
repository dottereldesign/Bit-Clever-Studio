export type FolderView={size?:number;sort?:string};
export type FolderViews=Record<string,Record<string,FolderView>>;
export const folderSorts=['recent','name','shuffle','rating-high','rating-low'];
export function folderViewLocation(route:{project:number|null;projectFolder:number|'unfiled'|null;generalKey:string|null;category:string|null;boardView:string|null;resourcesView:boolean;videoView:boolean;linksOnly:boolean;source:string;recent:string|null}):[string,string]{
 if(route.generalKey)return [JSON.stringify(['shortcut',route.generalKey]),'root'];
 if(route.project!==null)return [`project:${route.project}`,route.projectFolder===null?'root':typeof route.projectFolder==='number'?`folder:${route.projectFolder}`:'unfiled'];
 if(route.recent)return [JSON.stringify(['recent',route.recent]),'root'];
 if(route.boardView)return [JSON.stringify(['board',route.boardView]),'root'];
 if(route.resourcesView)return ['resources','root'];
 if(route.videoView)return ['videos','root'];
 if(route.linksOnly)return ['links','root'];
 if(route.source)return [JSON.stringify(['source',route.source,route.category]),'root'];
 return [JSON.stringify(['category',route.category]),'root'];
}
export function updateFolderView(previous:FolderViews,group:string,scope:string,value:FolderView):FolderViews{
 return {...previous,[group]:{...previous[group],[scope]:{...previous[group]?.[scope],...value}}};
}
