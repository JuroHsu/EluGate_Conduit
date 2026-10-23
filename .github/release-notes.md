## EluGate Conduit v0.2.1

### What's new / 更新內容

- Correct the Windows installer publisher and executable company name from `com` to **伊露科技有限公司 Elu Technology Ltd.**
- Traditional Chinese users can install `EluGate-Conduit_0.2.1_x64_zh-TW.msi`. Updating to this version refreshes the publisher shown in Windows Installed apps.
- MSI is provided in Traditional Chinese to preserve the bilingual publisher with the installer's supported code page. The EXE installer continues to offer all supported installer languages.
- Includes all v0.2.0 features and fixes, with Windows, macOS, and Linux installers.

- 將 Windows 安裝程式的發行者及執行檔公司名稱，從 `com` 修正為 **伊露科技有限公司 Elu Technology Ltd.**。
- 繁體中文使用者可下載 `EluGate-Conduit_0.2.1_x64_zh-TW.msi`；安裝更新後，Windows「已安裝的應用程式」會顯示修正後的發行者。
- MSI 提供繁體中文版，使用支援完整中英文公司名稱的編碼，並補齊中文安裝提示；EXE 安裝程式仍保留全部支援的語言。
- 包含 v0.2.0 的全部功能與修正，並提供 Windows、macOS 與 Linux 安裝檔。

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
