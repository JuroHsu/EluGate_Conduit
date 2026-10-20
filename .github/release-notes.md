## Download / 下載安裝檔

| Platform / 平台 | File / 檔案 |
| --- | --- |
| Windows 64-bit (recommended / 建議) | `*_x64-setup.exe` — installer with a language selector / 可選擇語言的安裝程式 |
| Windows 64-bit MSI | `*_x64_*.msi` — select your language, e.g. `zh-TW` or `en-US` / 選擇對應語言 |
| macOS Apple Silicon + Intel | `*_universal.dmg` — universal application / 通用版應用程式 |

Expand **Assets** below to download an installer. **Source code (zip/tar.gz)**
contains source files, not an installer. `SHA256SUMS.txt` contains SHA-256 checksums
for the installer assets.

展開下方 **Assets** 下載安裝檔。**Source code (zip/tar.gz)** 是原始碼，
不是安裝程式。`SHA256SUMS.txt` 提供安裝檔的 SHA-256 校驗值。

### Requirements and signing / 系統需求與簽章

- Install an OpenSSH client. Windows also requires Microsoft Edge WebView2 Runtime.
- Windows installers do not currently have a publisher code-signing certificate.
  Windows may show an unknown-publisher or reputation warning.
- The macOS app is ad-hoc signed, not Developer ID signed or notarized by Apple.
  macOS may block opening it. macOS behavior still needs end-to-end validation.
- These are early development builds; review the README's known limitations.

- 請安裝 OpenSSH 用戶端；Windows 另需 Microsoft Edge WebView2 Runtime。
- Windows 安裝檔目前沒有發行者程式碼簽章，可能出現未知發行者或信譽警告。
- macOS 目前僅使用 ad-hoc 簽章，尚無 Developer ID 簽章與 Apple 公證，
  系統可能阻擋開啟；macOS 行為仍待端到端驗證。
- 目前為早期開發版本，使用前請閱讀 README 的已知限制。

Released under the MIT License / 採 MIT 授權開源。

**伊露科技有限公司 Elu Technology Ltd.**  
[https://elu.com.tw/](https://elu.com.tw/) · [service@elu.com.tw](mailto:service@elu.com.tw)
