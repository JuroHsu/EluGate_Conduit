import { useState } from "react";
import logo from "../assets/logo.svg";
import { TunnelsPage } from "./TunnelsPage";
import { HostsPage } from "./HostsPage";
import { LogsPage } from "./LogsPage";
import { PrefsPage } from "./PrefsPage";
import { useTunnels } from "../hooks";
import { useI18n } from "../i18n";

type Page = "tunnels" | "hosts" | "logs" | "prefs";

const NAV: { id: Page; icon: string }[] = [
  { id: "tunnels", icon: "⇄" },
  { id: "hosts", icon: "▤" },
  { id: "logs", icon: "≡" },
  { id: "prefs", icon: "⚙" },
];

export function MainWindow() {
  const { t } = useI18n();
  const [page, setPage] = useState<Page>("tunnels");
  const [logFilter, setLogFilter] = useState<string | null>(null);
  const views = useTunnels();
  const connected = views?.filter((v) => v.runtime.status === "connected").length ?? 0;

  return (
    <div className="app">
      <nav className="sidebar" aria-label={t("nav.label")}>
        <div className="brand">
          <img src={logo} alt="" width={36} height={36} />
          <div>
            <div className="brand-name">EluGate Conduit</div>
            <div className="brand-sub">{connected ? t("brand.connected", { count: connected }) : t("brand.none")}</div>
          </div>
        </div>
        {NAV.map((n) => (
          <button
            key={n.id}
            className={`nav-item ${page === n.id ? "active" : ""}`}
            aria-current={page === n.id ? "page" : undefined}
            onClick={() => {
              setPage(n.id);
              if (n.id === "logs") setLogFilter(null);
            }}
          >
            <span className="nav-icon" aria-hidden>
              {n.icon}
            </span>
            {t(`nav.${n.id}`)}
          </button>
        ))}
      </nav>
      <main className="content">
        {page === "tunnels" && (
          <TunnelsPage
            views={views}
            onShowLogs={(id) => {
              setLogFilter(id);
              setPage("logs");
            }}
          />
        )}
        {page === "hosts" && <HostsPage />}
        {page === "logs" && <LogsPage views={views ?? []} filter={logFilter} onFilter={setLogFilter} />}
        {page === "prefs" && <PrefsPage />}
      </main>
    </div>
  );
}
