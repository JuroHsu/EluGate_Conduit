## EluGate Conduit v0.2.0

### What's new / 更新內容

- Add Linux x64 DEB, RPM, and AppImage packages, with tray menu integration.
- Fix English appearing on startup when Traditional Chinese or the system language is selected, and synchronize language changes across windows.
- Add **SSH Hosts → Add identity file** to generate `~/.ssh/id_ed25519` and its public key without overwriting existing files. Generated keys have no passphrase; add the public key to your server's `authorized_keys`.
- Add a native file picker for **IdentityFile** when creating or editing SSH hosts.
- Fix Windows launch-at-login paths containing spaces, repair existing unquoted entries, and respect Windows startup enable/disable state.

- 新增 Linux x64 的 DEB、RPM 與 AppImage 安裝檔，以及系統匣選單整合。
- 修正選擇繁體中文或跟隨系統時，啟動仍顯示英文的問題，並同步所有視窗的語言變更。
- 新增「**SSH 主機 → 新增身分檔**」，產生 `~/.ssh/id_ed25519` 與公鑰，不覆寫既有檔案。產生的金鑰未設定密碼；請將公鑰加入伺服器的 `authorized_keys`。
- 新增／編輯 SSH 主機的 **IdentityFile** 可使用原生檔案選擇器。
- 修正 Windows 登入啟動路徑含空白時的設定，自動修復既有未加引號項目，並正確讀取 Windows 啟動啟用／停用狀態。

## Download / 下載安裝檔

| Platform / 平台 | File / 檔案 |
| --- | --- |
| Windows 64-bit (recommended / 建議) | `*_x64-setup.exe` — installer with a language selector / 可選擇語言的安裝程式 |
| Windows 64-bit MSI | `*_x64_*.msi` — select your language, e.g. `zh-TW` or `en-US` / 選擇對應語言 |
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
