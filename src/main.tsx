import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import { MainWindow } from "./views/MainWindow";
import { Panel } from "./views/Panel";
import { AuthDialog } from "./views/AuthDialog";
import { I18nProvider } from "./i18n";

const params = new URLSearchParams(window.location.search);
const view = params.get("view") ?? "main";
document.body.dataset.view = view;

// 桌面應用不使用瀏覽器右鍵選單（輸入框除外）
window.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest("input, textarea, pre, code")) e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <I18nProvider>
      {view === "panel" ? <Panel /> : view === "auth" ? <AuthDialog id={params.get("id") ?? ""} /> : <MainWindow />}
    </I18nProvider>
  </React.StrictMode>,
);
