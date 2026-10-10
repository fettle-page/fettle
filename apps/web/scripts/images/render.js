// Page code run by playwright-cli; render.mjs replaces __ROOT__ with the repo root first.
async (page) => {
  const root = "__ROOT__";
  const here = `file:///${root}/apps/web/scripts/images`;
  const out = `${root}/apps/web/public`;
  await page.setViewportSize({ width: 1200, height: 630 });
  for (const name of ["site", "share"]) {
    await page.goto(`${here}/${name}.html`);
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: `${out}/og/${name}.png` });
  }
  await page.setViewportSize({ width: 180, height: 180 });
  await page.goto(`${here}/touch.html`);
  await page.screenshot({ path: `${out}/apple-touch-icon.png` });
  return "rendered og/site.png, og/share.png, apple-touch-icon.png";
}
