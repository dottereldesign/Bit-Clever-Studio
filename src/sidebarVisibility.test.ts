import test from 'node:test';
import assert from 'node:assert/strict';
import {readHiddenSections,mergeVisibleOrder} from './sidebarVisibilityData';
test('visibility preferences tolerate damaged storage and ignore unknown sections',()=>{
  assert.deepEqual(readHiddenSections('invalid'),[]);
  assert.deepEqual(readHiddenSections('{}'),[]);
  assert.deepEqual(readHiddenSections('["Categories","Clients","Categories",null,"old"]'),['Categories','Clients']);
});
test('visible reordering preserves hidden positions for later restoration',()=>{
  assert.deepEqual(mergeVisibleOrder(['Marketing','Leads','Categories','Components'],['Components','Marketing','Categories']),['Components','Leads','Marketing','Categories']);
  assert.deepEqual(mergeVisibleOrder(['Marketing','Leads'],[]),['Marketing','Leads']);
});
