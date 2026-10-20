// Optional documentation tool. See docs/screenshots/README.md for setup.
import { createRequire } from "node:module";
import { mkdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const root = new URL("../", import.meta.url);
const output = new URL("docs/screenshots/", root);
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  channel: process.env.SCREENSHOT_BROWSER || undefined,
});
try {
  for (const lang of ["en", "zh-TW"]) {
    const strings = JSON.parse(await readFile(new URL(`locales/${lang}.json`, root), "utf8"));
    const page = await browser.newPage({
      viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1, colorScheme: "light",
    });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(`http://127.0.0.1:1420/docs/preview/?lang=${lang}`);
    await page.getByRole("cell", { name: "Web Dashboard", exact: true }).waitFor();
    await page.evaluate(() => document.fonts.ready);
    const capture = (name) => page.screenshot({
      path: fileURLToPath(new URL(`${name}-${lang}.png`, output)),
      animations: "disabled",
    });
    await capture("tunnels");
    await page.getByRole("button", { name: strings["tunnels.add"], exact: true }).click();
    await page.getByLabel(strings["field.name"], { exact: true }).fill("Web Dashboard");
    await page.getByLabel(strings["field.sshHost"], { exact: true }).fill("demo-web");
    await page.getByText("demo@demo-web.example.com:22", { exact: false }).waitFor();
    await capture("tunnel-editor");
    await page.getByRole("button", { name: strings["common.cancel"], exact: true }).click();
    await page.getByRole("button", { name: strings["nav.hosts"], exact: false }).click();
    await page.getByRole("option", { name: "demo-web", exact: true }).click();
    await page.getByRole("cell", { name: "demo-web.example.com", exact: true }).waitFor();
    await capture("hosts");
    await page.setViewportSize({ width: 340, height: 460 });
    await page.goto(`http://127.0.0.1:1420/docs/preview/?lang=${lang}&view=panel`);
    await page.getByText("Web Dashboard", { exact: true }).waitFor();
    await page.evaluate(() => document.fonts.ready);
    await capture("panel");
    if (errors.length) throw new Error(errors.join("\n"));
    await page.close();
    console.log(`Captured ${lang}: tunnels, tunnel editor, hosts, panel.`);
  }
} finally {
  await browser.close();
}
