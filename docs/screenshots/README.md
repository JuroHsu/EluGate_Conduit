# Documentation screenshots / 文件截圖

These PNGs show the application's real React components rendered in Chromium
with synthetic documentation fixtures, not live SSH connections. Host names,
paths, and connection states are examples. No personal SSH configuration is read
or modified. Desktop window borders are omitted.

這些 PNG 是以 Chromium 擷取專案實際 React 元件的畫面，使用文件專用範例資料，
並非真實 SSH 連線。主機名稱、路徑與連線狀態均為展示用途，不會讀取或修改個人
SSH 設定，也不包含桌面視窗外框。

## Regenerate / 重新產生

From the repository root, install the optional capture dependency:

在專案根目錄安裝選用的截圖工具：

```bash
npm ci
npm install --no-save --package-lock=false playwright
npx playwright install chromium
npm run dev -- --host 127.0.0.1
```

In a second terminal / 在另一個終端機執行：

```bash
node scripts/capture-screenshots.mjs
```

Alternatively, set `SCREENSHOT_BROWSER=msedge` or `SCREENSHOT_BROWSER=chrome`
to use an already installed browser instead of downloading Chromium.

也可設定環境變數 `SCREENSHOT_BROWSER=msedge` 或 `SCREENSHOT_BROWSER=chrome`，
使用已安裝的瀏覽器，省略下載 Chromium。

The script captures English and Traditional Chinese at 1280 × 800, and the quick
panel at 340 × 460. Review the images before committing. The preview is available
at `http://127.0.0.1:1420/docs/preview/?lang=en` or `?lang=zh-TW`.
It is not included in the normal production build; unsupported operations fail
instead of opening a real connection.

腳本會產生英文與繁體中文的 1280 × 800 主視窗，以及 340 × 460 快速面板。
提交前請檢查圖片。預覽入口不會加入一般正式版本，且不支援的操作會回報錯誤，
不會建立真實連線。
