import {spawnSync} from 'node:child_process';
import {mkdir, rm} from 'node:fs/promises';
import {resolve} from 'node:path';
const directory=resolve('.tools/contracts');
await mkdir(directory,{recursive:true});
const output=resolve(directory,'native-recovery.json');
// Never let a failed producer leave an old fixture looking like a live contract.
await rm(output,{force:true});
const env={...process.env,DESIGN_CONTRACT_OUTPUT:output};
for(const args of [
  ['scripts/desktop.mjs','test','recovery_contract_round_trip'],
  ['node_modules/tsx/dist/cli.mjs','--test','tests/recovery-contract.test.ts'],
]){
  const result=spawnSync(process.execPath,args,{env,stdio:'inherit',windowsHide:true});
  if(result.error)throw result.error;
  if(result.status!==0)process.exit(result.status||1);
}
