import {spawnSync} from 'node:child_process';
const repetitions=Number(process.env.DESIGN_PERF_REPETITIONS||3);
if(!Number.isInteger(repetitions)||repetitions<1||repetitions>10)throw Error('Use 1–10 repetitions');
const label=process.env.DESIGN_PERF_LABEL||'profile';
if(!/^[a-z0-9_-]+$/i.test(label))throw Error('Invalid performance label');
// Sequential runs keep each browser profile isolated and avoid port/CPU contention.
for(const items of [4000,20000])for(let run=1;run<=repetitions;run++){
  const result=spawnSync(process.execPath,['scripts/check-grid-return.mjs','performance'],{
    env:{...process.env,DESIGN_PERF_ITEMS:String(items),DESIGN_PERF_LABEL:`${label}-${items}-${run}`},
    stdio:'inherit',windowsHide:true,
  });
  if(result.error)throw result.error;
  if(result.status!==0)process.exit(result.status||1);
}
