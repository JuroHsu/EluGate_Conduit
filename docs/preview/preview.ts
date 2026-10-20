// Documentation-only fixture: render the actual app UI without reading local
// SSH settings, starting SSH processes, or persisting application data.
// This entry is not imported by the desktop application's production entry.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

const language = new URLSearchParams(location.search).get("lang") === "zh-TW" ? "zh-TW" : "en";
const at = "2026-10-20T12:00:00Z";
const views = [
  ["web", "Web Dashboard", "demo-web", 16000, 80, "connected"],
  ["db", "PostgreSQL", "demo-db", 15432, 5432, "disconnected"],
  ["cache", "Redis", "demo-cache", 16379, 6379, "disconnected"],
].map(([id, name, sshHost, localPort, destPort, status]) => ({
  tunnel: {
    id, name, sshHost, localPort, destPort,
    sshConfigFile: null, bindAddress: "127.0.0.1", destHost: "127.0.0.1",
    autoReconnect: true, connectOnLaunch: false,
  },
  runtime: { status, since: at, attempt: 0, connectedAt: status === "connected" ? at : null },
  pendingChanges: false,
}));

mockWindows("main", "panel");
mockIPC((command, payload) => {
  switch (command) {
    case "get_language": return language;
    case "get_state": return views;
    case "get_prefs": return { language };
    case "list_hosts": return {
      path: "~/.ssh/config",
      exists: true,
      hosts: views.map(({ tunnel }, i) => ({
        alias: tunnel.sshHost, source: "~/.ssh/config", line: i * 6 + 1, editable: true,
      })),
    };
    case "effective_host": return [
      ["hostname", `${String(payload?.alias ?? "demo-web")}.example.com`],
      ["user", "demo"], ["port", "22"], ["identityfile", "~/.ssh/id_ed25519"],
      ["proxyjump", "none"], ["serveraliveinterval", "15"], ["stricthostkeychecking", "ask"],
    ];
    case "validate_tunnel": return { errors: {}, warnings: [] };
    // Writes and connection operations intentionally fail: this is a visual preview.
    default: throw new Error(`Documentation preview does not implement ${command}`);
  }
}, { shouldMockEvents: true });

await import("../../src/main");
