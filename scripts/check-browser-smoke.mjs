import {spawnSync} from 'node:child_process';

// Each fixture owns its loopback server and disposable Chrome profile.
// Run sequentially; stop on failure so CI cannot report a partial suite as green.
const fixtures = ['startup', 'categories', 'app-history', 'project-folders', 'project-folder-drag',
  'thumbnail-enlargement', 'grid-return', 'library-isolation', 'refresh-consistency', 'preference-recovery', 'accessibility', 'workflow-tools', 'library-export'];
for (const fixture of fixtures) {
  console.log(`\nBrowser smoke: ${fixture}`);
  const result = spawnSync(process.execPath, ['scripts/check-grid-return.mjs', fixture], {
    stdio: 'inherit', windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
