// 由 design/*.svg 產生 Tauri 所需的 App 圖示與系統匣 PNG。
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";

const tauri = (args) =>
  execFileSync(process.platform === "win32" ? "npx.cmd" : "npx", ["tauri", ...args], {
    stdio: "inherit",
    shell: process.platform === "win32",
  });

tauri(["icon", "design/app-icon.svg", "-o", "src-tauri/icons"]);

const tmp = "src-tauri/icons/.tray-tmp";
mkdirSync("src-tauri/icons/tray", { recursive: true });
for (const status of ["idle", "connected", "reconnecting", "error"]) {
  rmSync(tmp, { recursive: true, force: true });
  tauri(["icon", `design/tray-${status}.svg`, "-o", tmp, "--png", "64"]);
  copyFileSync(join(tmp, "64x64.png"), `src-tauri/icons/tray/${status}.png`);
}
rmSync(tmp, { recursive: true, force: true });
