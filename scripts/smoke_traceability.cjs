// Run with Playwright on NODE_PATH and locally installed Edge; no network required.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromium } = require('playwright');

(async () => {
  const report = path.resolve(process.argv[2] || 'examples/traceability/generated/demo.html');
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  try {
    const page = await browser.newPage({ acceptDownloads: true });
    const errors = [], requests = [];
    page.on('pageerror', e => errors.push(e.message));
    page.on('request', r => { if (/^https?:/.test(r.url())) requests.push(r.url()); });
    for (const viewport of [{ width: 1440, height: 1000 }, { width: 390, height: 844 }]) {
      await page.setViewportSize(viewport);
      await page.goto(pathToFileURL(report).href);
      assert.equal(await page.locator('#device-count').innerText(), '10');
      await page.selectOption('#anomaly', 'retest_different');
      assert.ok(await page.locator('#matrix tbody tr').count() >= 3);
      assert.ok(await page.locator('#matrix .retest_different').count() >= 3);
      await page.click('#reset');
      await page.locator('#wafer').fill('W1');
      await page.locator('#x').fill('2');
      await page.locator('#y').fill('1');
      await page.selectOption('#step', '1');
      assert.equal(await page.locator('#matrix tbody tr').count(), 1);
      assert.equal(await page.locator('#matrix th').count(), 2);
      assert.match(await page.locator('#matrix tbody').innerText(), /缺失/);
      await page.locator('#matrix tbody button').click();
      assert.equal(await page.locator('#detail').isVisible(), true);
      assert.equal(await page.locator('#comparison .step-card').count(), 4);
      assert.equal(await page.locator('#attempts tbody tr').count(), 2);
      assert.match(await page.locator('#attempts tbody').innerText(), /SHA-256/);
      await page.click('#reset');
      await page.locator('#x').fill('1');
      await page.locator('#matrix tbody button').first().click();
      assert.equal(await page.locator('#attempts tbody tr').count(), 6);
      assert.match(await page.locator('#comparison').innerText(), /判定变化/);
      await page.click('#reset');
      await page.selectOption('#anomaly', 'order_unknown');
      await page.locator('#matrix tbody button').first().click();
      assert.match(await page.locator('#comparison').innerText(), /顺序未知 · 无最终判定/);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 2), false);
    }
    const downloadPromise = page.waitForEvent('download');
    await page.click('#export');
    const download = await downloadPromise;
    const evidence = JSON.parse(fs.readFileSync(await download.path(), 'utf8'));
    assert.equal(evidence.devices.length, 10); // Export is independent of active filters.
    assert.equal(evidence.schema, 'traceability-v1');
    assert.ok(Object.values(evidence.sources).some(paths => paths.length === 2));
    assert.equal(evidence.identity_map.length, 1);
    await page.click('#reset');
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: path.join(path.dirname(report), 'browser-smoke.png'), fullPage: true });
    assert.deepEqual(errors, []);
    assert.deepEqual(requests, []);
    console.log('PASS: desktop/mobile filters, missing/difference highlights, drilldown, unknown order, full JSON export, zero external requests');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
