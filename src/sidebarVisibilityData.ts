export const sidebarSections = [
  ['Marketing','Marketing'], ['Leads','Leads'], ['Clients','Organisations'],
  ['My Business','My Business'], ['Projects','Projects'], ['General','General'],
  ['Categories','Design Inspo'], ['Components','App Components'], ['Code','Code'],
  ['Analytics','Analytics'], ['Research','Research'],
] as const;

export function readHiddenSections(raw:string|null):string[] {
  try {
    const value:unknown = JSON.parse(raw??'[]');
    return Array.isArray(value)?[...new Set(value.filter((name):name is string=>typeof name==='string'&&sidebarSections.some(([id])=>id===name)))]:[];
  } catch { return []; }
}

// Reordering visible sections leaves hidden sections in their saved slots.
export function mergeVisibleOrder(all:string[],visible:string[]):string[] {
  const shown=new Set(visible);let index=0;
  return all.map(name=>shown.has(name)?visible[index++]:name);
}
