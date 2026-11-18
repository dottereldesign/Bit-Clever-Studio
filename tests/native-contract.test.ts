import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {analyticsData, normalizeAnalytics} from '../src/analyticsData';
import {recordedRevenue, defaultOverview} from '../src/homeOverviewData';
import type {Library} from '../src/model';

// Native db::load output from a disposable database, not a JS reconstruction.
const library: Library = JSON.parse(readFileSync(new URL('./fixtures/native-won-lead.json', import.meta.url), 'utf8'));
test('native won-lead JSON produces the same NZD value in period Analytics and Home', () => {
  const closed = library.studio![0].closed_at!;
  const now = new Date(closed + 60_000);
  const analytics = analyticsData(library, normalizeAnalytics({months: 3}), now);
  const home = recordedRevenue(library, defaultOverview, now);
  assert.equal(analytics.wonValue, 1200);
  assert.equal(analytics.missingClose, 0);
  assert.equal(analytics.trend.reduce((sum, month) => sum + month.value, 0), 1200);
  assert.equal(home.total, 1200);
  assert.equal(home.months.at(-1)?.value, 1200);
  assert.deepEqual(library.undo_entries, [{id: 1, label: 'Edit studio record'}, {id: 2, label: 'Edit studio record'}]);
});
