export const activeGroups=['sidebar','tabs','buttons'] as const;
export const activeParts=['background','text','icon','count'] as const;
export type ActiveGroup=typeof activeGroups[number];
export type ActivePart=typeof activeParts[number];
export type ActiveColours=Partial<Record<ActiveGroup,Partial<Record<ActivePart,string>>>>;
export function normalizeActiveColours(value:unknown):ActiveColours|undefined {
 if(!value||typeof value!=='object'||Array.isArray(value))return undefined;
 const result:ActiveColours={};
 for(const group of activeGroups){
  const entry=(value as Record<string,unknown>)[group];
  if(!entry||typeof entry!=='object'||Array.isArray(entry))continue;
  result[group]={};
  for(const part of activeParts){const colour=(entry as Record<string,unknown>)[part];if(typeof colour==='string'&&/^#[0-9a-f]{6}$/i.test(colour))result[group]![part]=colour;}
 }
 return result;
}
export function readActiveColours():ActiveColours {try{return normalizeActiveColours(JSON.parse(localStorage.getItem('active-colours')??'{}'))??{};}catch{return {};}}
