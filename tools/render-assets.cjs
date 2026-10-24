// Draw the SVGs in `assets/` to the PNGs and the `.ico` that go beside them.
//
// Chromium rather than a converter, because what we want on disk is what a browser shows:
// the same font fallback, the same antialiasing, the same rounded corners.
//
//   node tools/render-assets.cjs
//
// Needs `playwright` (the container the project builds in has it) and ImageMagick's
// `convert` for the Windows icon.
const { chromium } = require('playwright');
const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const root = path.join(__dirname, '..', 'assets');

async function draw(page, svg, png, width, height) {
  await page.setViewportSize({ width, height });
  await page.setContent(
    `<style>html,body{margin:0;padding:0}svg{display:block;width:${width}px;height:${height}px}</style>` +
      fs.readFileSync(path.join(root, svg), 'utf8'),
    { waitUntil: 'load' },
  );
  // Far enough into the animation that everything has arrived.
  await page.waitForTimeout(4000);
  await page.screenshot({ path: path.join(root, png) });
  console.log(`${png}  ${width}x${height}`);
}

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  await draw(page, 'banner.svg', 'banner.png', 1280, 640);
  await draw(page, 'logo.svg', 'logo.png', 880, 236);
  await draw(page, 'icon.svg', 'icon-256.png', 256, 256);
  await browser.close();

  const source = path.join(root, 'icon-256.png');
  for (const size of [16, 32, 48, 64, 128]) {
    execFileSync('convert', [source, '-resize', `${size}x${size}`, path.join(root, `icon-${size}.png`)]);
    console.log(`icon-${size}.png`);
  }
  execFileSync('convert', [source, '-define', 'icon:auto-resize=256,128,64,48,32,16', path.join(root, 'scmd.ico')]);
  console.log('scmd.ico');
})();
