# 發布安裝檔 / Publishing installers

工作流程：[`.github/workflows/release.yml`](../.github/workflows/release.yml)。

## 自動建置的內容

| 平台 | Release 附件 |
| --- | --- |
| Windows x64 | NSIS `.exe`（可選語言）與各語言的 `.msi` |
| macOS Apple Silicon + Intel | 通用版 `.dmg` |
| Linux x64 (Ubuntu 22.04 build) | `.deb`、`.rpm`、`.AppImage` |
| 校驗資訊 | `SHA256SUMS.txt` |

推送 `v*` 標籤會執行測試與建置。三個平台全部成功後，工作流程才建立草稿、
上傳安裝檔與 SHA-256 清單，再公開 Release。`-beta.1` 等版本會標記為
pre-release。已公開的 Release 不會被重跑覆寫，修正請發布新版本。

工作流程使用 GitHub 自動提供的 `GITHUB_TOKEN`，不需要自行新增 PAT。
只有發布工作具有 `contents: write`，建置工作只有讀取權限。

## 首次發布流程（以 v0.1.0 為例）

以下保留首次發布 `0.1.0` 的流程範例；目前版本為 `0.2.0`。首次設定時，先把工作流程、檢查腳本與文件提交到 GitHub：

```bash
git add .github scripts/check-release-version.mjs scripts/check-release-version.test.mjs docs/RELEASING.md README.md README.zh-TW.md
git commit -m "ci: build installers and publish GitHub releases"
git push origin main
```

如果尚未要公開版本，可以先到 **Actions → Build installers and release →
Run workflow**，選擇 `main` 分支執行。分支上的手動執行只產生 Actions artifacts，
不會公開 Release。建置成功後，可在該次執行頁面的 **Artifacts** 下載 ZIP，
解壓縮後測試其中的安裝程式。測試產物保留 14 天。

確認要發布後，建立標籤：

```bash
git tag -a v0.1.0 -m "EluGate Conduit v0.1.0"
git push origin v0.1.0
```

在 [Actions](https://github.com/JuroHsu/EluGate_Conduit/actions) 查看進度。
成功後使用者可直接從 [Releases](https://github.com/JuroHsu/EluGate_Conduit/releases)
的 **Assets** 下載安裝檔，不需要登入 Actions 或自行編譯。

若儲存庫或組織停用了 Actions，需要先允許執行工作流程。發布權限若被組織政策限制，
請確認該政策允許此工作流程的 `contents: write`。

## 發布下一個版本

例如發布 `0.2.0`：

1. 執行 `npm version 0.2.0 --no-git-tag-version`，更新 `package.json` 與 `package-lock.json`。
2. 將 `src-tauri/tauri.conf.json` 的頂層 `version` 和 `src-tauri/Cargo.toml` 的
   `[package].version` 都改為 `0.2.0`。
3. 執行 `cargo check --manifest-path src-tauri/Cargo.toml`，讓 Cargo 同步根套件的
   `Cargo.lock` 版本。
4. 執行 `node scripts/check-release-version.mjs` 確認版本一致。
5. 更新 [Release 說明](../.github/release-notes.md)，提交所有版本變更並推送。
6. 建立並推送 `v0.2.0` 標籤。

標籤必須與程式版本完全相符。例如 `v0.2.0` 不能對應內容仍為 `0.1.0` 的提交，
否則工作流程會停止。測試版可以使用 `0.2.0-beta.1` 與標籤 `v0.2.0-beta.1`。

## 簽章與平台驗證

- Windows 安裝檔目前未使用發行者程式碼簽章，可能出現未知發行者或 SmartScreen 提示。
- macOS 在 CI 使用 `APPLE_SIGNING_IDENTITY=-` 進行 ad-hoc 簽章，
  不等於 Developer ID 簽章或 Apple 公證，下載後仍可能被 Gatekeeper 阻擋。
- 若要改善正式散布的安裝體驗，請依 Tauri 官方的
  [Windows 簽章](https://v2.tauri.app/distribute/sign/windows/)與
  [macOS 簽章／公證](https://v2.tauri.app/distribute/sign/macos/)指引準備憑證，
  使用 GitHub Actions secrets 傳入；不要把私鑰加入儲存庫。
- CI 建置與單元測試成功，不代表安裝、SSH 連線與平台整合已完成端到端驗證。
  發布前仍應在目標系統實測。

## 失敗與重跑

- 版本錯誤：先修正版本或建立正確標籤，不要移動已公開版本的標籤。
- 建置失敗：查看失敗的 job；尚未進行發布時，Release 不會公開。
- 上傳失敗：可能留下草稿 Release。使用該次執行的 **Re-run all jobs** 重試。
- 已公開版本：工作流程會拒絕覆寫，請增加版本號後發布。
- 手動試跑時只看到 artifacts：這是預期行為；推送版本標籤才會自動公開 Release。

## English quick reference

Push a `v<version>` tag to build Windows x64 EXE/MSI installers and a universal
macOS DMG, plus Linux x64 DEB/RPM/AppImage packages. All matrix jobs must succeed before a release is published.
Branch-based manual runs only upload Actions artifacts (retained for 14 days).

Keep the versions in `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, and the root package in `src-tauri/Cargo.lock` in sync.
The workflow rejects mismatched tags and refuses to replace a public release.
Versions such as `0.2.0-beta.1` are published as prereleases.

No personal access token is needed. GitHub provides the release job's token.
Windows packages are not publisher-signed; macOS packages are only ad-hoc signed,
not notarized. Signing setup and end-to-end platform validation remain separate
release tasks.

See the [Tauri GitHub Actions guide](https://v2.tauri.app/distribute/pipelines/github/)
and [GitHub artifact documentation](https://docs.github.com/en/actions/tutorials/store-and-share-data)
for the underlying build and artifact mechanisms.
