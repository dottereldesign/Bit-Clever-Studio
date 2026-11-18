import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createMembershipIndex,filterItems,type Library,type Filters} from '../src/model';
import {normalizePreference} from '../src/preferenceSchema';
import {AppHistory} from '../src/appHistory';

// The Windows contract command replaces the frozen fixture with fresh Rust output.
const wire=JSON.parse(readFileSync(process.env.DESIGN_CONTRACT_OUTPUT||new URL('./fixtures/native-recovery.json',import.meta.url),'utf8')) as {
  imported:Library;rated:Library;undone:Library;redone:Library;preferences:Record<string,unknown>;
};
const defaults:Filters={search:'',type:'all',tags:[],source:'',project:null,linksOnly:false,sort:'newest',seed:0};

test('native duplicate recovery/import references resolve in frontend membership indexes',()=>{
  const lib=wire.imported,index=createMembershipIndex(lib.items);
  const project=lib.projects.find(p=>p.name==='Contract project')!;
  assert.notEqual(project.id,7,'import remaps a colliding project ID');
  const keeper=lib.items.find(i=>i.name==='Image 1')!;
  const folder=lib.project_folders!.find(f=>f.name==='References')!;
  assert.equal(folder.project_id,project.id);
  assert.equal(lib.project_folders!.find(f=>f.id===folder.parent_id)?.name,'Research');
  assert.ok(index.projectFolders.get(folder.id)?.includes(keeper));
  for(const category of ['Illustration','Components / Products']){
    assert.ok(filterItems(lib.items,{...defaults,category},index).includes(keeper));
  }
  assert.equal(lib.project_overviews![0].moodboard![0].item_id,keeper.id);
  assert.deepEqual(lib.client_records!.records[0].files,[keeper.id]);
  const privateLink=lib.items.find(i=>i.name==='Private link')!;
  assert.equal(privateLink.project_only,true);
  assert.ok(filterItems(lib.items,{...defaults,project:project.id},index).includes(privateLink));
  assert.ok(!index.designs.includes(privateLink));
  assert.deepEqual(keeper.favourites,[project.id]);
  assert.equal(project.count,3);
});

test('native rating undo/redo snapshots retain values and durable history shape',()=>{
  const rating=(lib:Library)=>lib.items.find(i=>i.name==='Private link')!.rating;
  assert.equal(rating(wire.imported),4);assert.equal(rating(wire.rated),5);
  assert.equal(rating(wire.undone),4);assert.equal(rating(wire.redone),5);
  assert.equal(wire.undone.redo_count,1);assert.ok(wire.undone.redo_label);
  assert.equal(wire.redone.redo_count,0);
  assert.equal(wire.rated.undo_count,wire.rated.undo_entries!.length);
  assert.equal(wire.rated.undo_entries!.at(-1)?.label,wire.rated.undo_label);
  assert.equal(wire.redone.undo_label,wire.rated.undo_label);
});

test('native remapped preferences survive frontend normalisation and preference undo/redo',async()=>{
  const project=wire.imported.projects.find(p=>p.name==='Contract project')!;
  const key=`project:${project.id}`;
  const colours=normalizePreference('colour-history',wire.preferences['colour-history']);
  assert.deepEqual(colours,[{hex:'#123456',projects:[key]}]);
  const tiles=normalizePreference('home-tiles',wire.preferences['home-tiles']) as {id:string}[];
  assert.deepEqual(tiles.map(tile=>tile.id),[key]);
  assert.deepEqual(normalizePreference('home-tiles',tiles),tiles);
  const history=new AppHistory();
  history.commit('Restore imported preferences',{'sidebar-width':260},{'sidebar-width':wire.preferences['sidebar-width']});
  assert.equal(history.read('sidebar-width',()=>0),305);
  await history.undo();assert.equal(history.read('sidebar-width',()=>0),260);
  await history.redo();assert.equal(history.read('sidebar-width',()=>0),305);
});
