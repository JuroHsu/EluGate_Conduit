import { invoke } from "@tauri-apps/api/core";

export type Status =
  | "disconnected"
  | "connecting"
  | "waitingAuth"
  | "connected"
  | "reconnecting"
  | "stopping"
  | "failed";

export interface Tunnel {
  id: string;
  name: string;
  sshHost: string;
  sshConfigFile?: string | null;
  bindAddress: string;
  localPort: number;
  destHost: string;
  destPort: number;
  autoReconnect: boolean;
  connectOnLaunch: boolean;
}

export interface DestCheck {
  ok: boolean;
  message: string;
  at: string;
}

export interface Runtime {
  status: Status;
  message?: string | null;
  errorKind?: string | null;
  since: string;
  attempt: number;
  nextRetryAt?: string | null;
  connectedAt?: string | null;
  destCheck?: DestCheck | null;
  applied?: Tunnel | null;
}

export interface TunnelView {
  tunnel: Tunnel;
  runtime: Runtime;
  pendingChanges: boolean;
}

export interface CmdError {
  code: string;
  message: string;
  conflicts?: { id: string; name: string; endpoint: string }[];
  fields?: Record<string, string>;
}

export interface Validation {
  errors: Record<string, string>;
  warnings: string[];
}

export interface Stage {
  status: "ok" | "failed" | "skipped";
  message: string;
}

export interface TestResult {
  ssh: Stage;
  forward: Stage;
  destination: Stage;
}

export interface LogEntry {
  id: number;
  ts: string;
  level: "debug" | "info" | "warn" | "error";
  tunnelId?: string | null;
  message: string;
}

export interface HostEntry {
  alias: string;
  source: string;
  line: number;
  editable: boolean;
}

export interface HostList {
  path: string;
  exists: boolean;
  hosts: HostEntry[];
}

export interface HostForm {
  alias: string;
  hostName: string;
  user: string;
  port: string;
  identityFile: string;
  proxyJump: string;
}

export interface Identity {
  privatePath: string;
  publicPath: string;
  publicKey: string;
}

export interface FileSnapshot {
  path: string;
  exists: boolean;
  content: string;
  hash: string;
}

export interface Preview {
  baseHash: string;
  content: string;
  diff: string;
}

export interface Prefs {
  sshPath?: string | null;
  sshConfigPath?: string | null;
  /** null 或 "auto" 代表跟隨系統 */
  language?: string | null;
}

export interface EnvInfo {
  ssh: { path?: string | null; version?: string | null; error?: string | null };
  dataDir: string;
  logDir: string;
  defaultConfig: string;
  autostart: boolean;
  version: string;
  platform: string;
}

export interface Prompt {
  id: string;
  tunnelId: string;
  tunnelName: string;
  kind: "hostkey" | "password" | "passphrase" | "confirm" | "other";
  message: string;
}

export const api = {
  getState: () => invoke<TunnelView[]>("get_state"),
  getPrefs: () => invoke<Prefs>("get_prefs"),
  validateTunnel: (tunnel: Tunnel) => invoke<Validation>("validate_tunnel", { tunnel }),
  saveTunnel: (tunnel: Tunnel) => invoke<Tunnel>("save_tunnel", { tunnel }),
  duplicateTunnel: (id: string) => invoke<Tunnel>("duplicate_tunnel", { id }),
  deleteTunnel: (id: string) => invoke<void>("delete_tunnel", { id }),
  connect: (id: string) => invoke<void>("connect_tunnel", { id }),
  disconnect: (id: string) => invoke<void>("disconnect_tunnel", { id }),
  switchTo: (id: string) => invoke<void>("switch_tunnel", { id }),
  restart: (id: string) => invoke<void>("restart_tunnel", { id }),
  stopAll: () => invoke<void>("stop_all"),
  checkDestination: (id: string) => invoke<DestCheck | null>("check_destination", { id }),
  testTunnel: (tunnel: Tunnel) => invoke<TestResult>("test_tunnel", { tunnel }),
  getLogs: (tunnelId?: string | null) => invoke<LogEntry[]>("get_logs", { tunnelId: tunnelId ?? null }),
  clearLogs: () => invoke<void>("clear_logs"),
  openLogDir: () => invoke<void>("open_log_dir"),
  listHosts: (path?: string | null) => invoke<HostList>("list_hosts", { path: path ?? null }),
  effectiveHost: (alias: string, path?: string | null) =>
    invoke<[string, string][]>("effective_host", { alias, path: path ?? null }),
  readSshConfig: (path?: string | null) => invoke<FileSnapshot>("read_ssh_config", { path: path ?? null }),
  hostForm: (alias: string, path?: string | null) => invoke<HostForm | null>("host_form", { alias, path: path ?? null }),
  previewHostForm: (form: HostForm, path?: string | null) =>
    invoke<Preview>("preview_host_form", { form, path: path ?? null }),
  previewSshConfig: (content: string, path?: string | null) =>
    invoke<Preview>("preview_ssh_config", { content, path: path ?? null }),
  saveSshConfig: (content: string, baseHash: string, path?: string | null) =>
    invoke<string | null>("save_ssh_config", { content, baseHash, path: path ?? null }),
  getEnv: () => invoke<EnvInfo>("get_env"),
  setPrefs: (prefs: Prefs) => invoke<void>("set_prefs", { prefs }),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  generateIdentity: () => invoke<Identity>("generate_identity"),
  selectIdentityFile: () => invoke<string | null>("select_identity_file"),
  getPrompt: (id: string) => invoke<Prompt | null>("get_prompt", { id }),
  answerPrompt: (id: string, value: string | null) => invoke<boolean>("answer_prompt", { id, value }),
  showMain: () => invoke<void>("show_main"),
  hidePanel: () => invoke<void>("hide_panel"),
  quit: () => invoke<void>("quit_app"),
};

export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as CmdError).message);
  return String(e);
}

export const isActive = (s: Status) => s !== "disconnected" && s !== "failed";

export const endpoint = (t: Pick<Tunnel, "bindAddress" | "localPort">) =>
  `${t.bindAddress.includes(":") ? `[${t.bindAddress}]` : t.bindAddress}:${t.localPort}`;

export const destination = (t: Pick<Tunnel, "destHost" | "destPort">) =>
  `${t.destHost.includes(":") ? `[${t.destHost}]` : t.destHost}:${t.destPort}`;

export function newTunnel(): Tunnel {
  return {
    id: "",
    name: "",
    sshHost: "",
    sshConfigFile: null,
    bindAddress: "127.0.0.1",
    localPort: 16000,
    destHost: "127.0.0.1",
    destPort: 80,
    autoReconnect: true,
    connectOnLaunch: false,
  };
}
