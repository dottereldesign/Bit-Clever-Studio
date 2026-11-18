import {spawnSync} from 'node:child_process';
import {resolve} from 'node:path';
const args=process.argv.slice(2),env={...process.env};
delete env.DESIGN_VERIFY_BACKUP;
let filter='disposable_restore_',flags=['--nocapture'];
if(args.length){
  if(args.length!==2||args[0]!=='--folder'||!args[1])throw Error('Usage: npm run verify:restore [-- --folder "PATH"]');
  env.DESIGN_VERIFY_BACKUP=resolve(args[1]);filter='verify_supplied_backup';flags=['--ignored','--nocapture'];
}
console.log(args.length?'Verifying the exported backup using a disposable restore. The source and active library are not changed.':'Running disposable restore verification against synthetic fixtures.');
const result=spawnSync(process.execPath,['scripts/desktop.mjs','test',filter,'--',...flags],{env,stdio:'inherit',windowsHide:true});
if(result.error)throw result.error;
process.exitCode=result.status??1;
