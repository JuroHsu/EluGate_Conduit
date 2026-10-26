## EluGate Conduit v0.2.2

### What's new / 更新內容

- Fix middle-mouse dragging moving the entire interface out of view by disabling browser autoscroll and keeping the outer window layout fixed.
- Preserve normal mouse-wheel scrolling in content lists, logs, dialogs, and the system tray panel. Scrolling at an area's boundary no longer propagates to its parent.
- Keep long authentication prompts accessible through scrolling within the authentication window.
- Includes all v0.2.1 features and fixes, with Windows, macOS, and Linux installers.

- 修正按住滾輪拖曳會將整個介面捲出視窗的問題，停用瀏覽器自動捲動並固定外層版面。
- 保留內容清單、日誌、對話框及系統匣面板的一般滾輪操作，捲動至邊界時不再連帶捲動外層內容。
- 驗證視窗中的較長提示仍可透過內部捲動完整閱讀。
- 包含 v0.2.1 的全部功能與修正，並提供 Windows、macOS 與 Linux 安裝檔。

## Download / 下載安裝檔

| Platform / 平台 | File / 檔案 |
| --- | --- |
| Windows 64-bit (recommended / 建議) | `*_x64-setup.exe` — installer with a language selector / 可選擇語言的安裝程式 |
| Windows 64-bit MSI | `*_x64_zh-TW.msi` — Traditional Chinese / 繁體中文版 |
| macOS Apple Silicon + Intel | `*_universal.dmg` — universal application / 通用版應用程式 |
| Linux x64 | `.deb`, `.rpm`, `.AppImage` — built on Ubuntu 22.04 / 以 Ubuntu 22.04 建置 |

Expand **Assets** below to download an installer. **Source code (zip/tar.gz)**
contains source files, not an installer. `SHA256SUMS.txt` contains SHA-256 checksums
for the installer assets.

展開下方 **Assets** 下載安裝檔。**Source code (zip/tar.gz)** 是原始碼，
不是安裝程式。`SHA256SUMS.txt` 提供安裝檔的 SHA-256 校驗值。

### Requirements and signing / 系統需求與簽章

- Install an OpenSSH client. Windows also requires Microsoft Edge WebView2 Runtime.
- Linux requires WebKitGTK 4.1 and AppIndicator support; GNOME may need an AppIndicator extension. Make AppImage files executable before launching.
- Windows installers do not currently have a publisher code-signing certificate.
  Windows may show an unknown-publisher or reputation warning.
- The macOS app is ad-hoc signed, not Developer ID signed or notarized by Apple.
  macOS may block opening it. macOS behavior still needs end-to-end validation.
- These are early development builds; review the README's known limitations.

- 請安裝 OpenSSH 用戶端；Windows 另需 Microsoft Edge WebView2 Runtime。
- Linux 需要 WebKitGTK 4.1 與 AppIndicator 支援；GNOME 可能需要擴充功能。AppImage 請先加上執行權限。
- Windows 安裝檔目前沒有發行者程式碼簽章，可能出現未知發行者或信譽警告。
- macOS 目前僅使用 ad-hoc 簽章，尚無 Developer ID 簽章與 Apple 公證，
  系統可能阻擋開啟；macOS 行為仍待端到端驗證。
- 目前為早期開發版本，使用前請閱讀 README 的已知限制。

Released under the MIT License / 採 MIT 授權開源。

**伊露科技有限公司 Elu Technology Ltd.**  
[https://elu.com.tw/](https://elu.com.tw/) · [service@elu.com.tw](mailto:service@elu.com.tw)
