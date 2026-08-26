import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type EngineState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "disconnecting";

export interface EngineStatus {
  state: EngineState;
  phase: string | null;
  gateway: string | null;
  detail: string | null;
  connectedSinceMs: number | null;
}

export interface LogRecord {
  tsMs: number;
  level: string;
  target: string;
  message: string;
}

export type Protocol = "masque" | "wireguard" | "gool";
export type ScanMode = "turbo" | "balanced" | "thorough" | "stealth" | "ironclad";
export type IpVersion = "v4" | "v6" | "dual";
export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

export interface NetherSettings {
  version: number;
  protocol: Protocol;
  scanMode: ScanMode;
  ipVersion: IpVersion;
  quickReconnect: boolean;
  autoConnect: boolean;
  alwaysOn: boolean;
  socksHost: string;
  socksPort: number;
  httpProxyEnabled: boolean;
  httpProxyPort: number;
  upstreamProxy: string | null;
  smartRouting: boolean;
  xraySocksPort: number;
  noizeProfile: string;
  masqueH2: boolean;
  ech: string | null;
  fragment: boolean;
  fragmentSize: string;
  fragmentDelay: string;
  dnsResolvers: string;
  validateSecs: number | null;
  startupSecs: number | null;
  reconnectSecs: number | null;
  disableDataCheck: boolean;
  keepaliveSecs: number | null;
  noProfileRetry: boolean;
  forcedPeer: string | null;
  forcedWgPeer: string | null;
  teamName: string | null;
  accessClientId: string | null;
  accessClientSecret: string | null;
  accessToken: string | null;
  useGatewayProxy: boolean;
  routeBlock: string | null;
  routeDirect: string | null;
  logLevel: LogLevel;
  perfProfile: string | null;
  tlsGroups: string | null;
}

export const DEFAULT_SETTINGS: NetherSettings = {
  version: 1,
  protocol: "masque",
  scanMode: "balanced",
  ipVersion: "v4",
  quickReconnect: true,
  autoConnect: false,
  alwaysOn: false,
  socksHost: "127.0.0.1",
  socksPort: 1819,
  httpProxyEnabled: false,
  httpProxyPort: 1820,
  upstreamProxy: null,
  smartRouting: false,
  xraySocksPort: 1817,
  noizeProfile: "firewall",
  masqueH2: false,
  ech: null,
  fragment: false,
  fragmentSize: "",
  fragmentDelay: "",
  dnsResolvers: "1.1.1.1,1.0.0.1",
  validateSecs: null,
  startupSecs: null,
  reconnectSecs: null,
  disableDataCheck: false,
  keepaliveSecs: null,
  noProfileRetry: false,
  forcedPeer: null,
  forcedWgPeer: null,
  teamName: null,
  accessClientId: null,
  accessClientSecret: null,
  accessToken: null,
  useGatewayProxy: false,
  routeBlock: null,
  routeDirect: null,
  logLevel: "info",
  perfProfile: null,
  tlsGroups: null,
};

export const PROTOCOLS: { value: Protocol; label: string; hint: string }[] = [
  { value: "masque", label: "MASQUE", hint: "Looks like normal HTTPS. Recommended." },
  { value: "wireguard", label: "WireGuard", hint: "Fast and lightweight." },
  { value: "gool", label: "Gool", hint: "WARP-in-WARP double tunnel." },
];

export const SCAN_MODES: { value: ScanMode; label: string; hint: string }[] = [
  { value: "turbo", label: "Turbo", hint: "Fastest scan, fewer candidates." },
  { value: "balanced", label: "Balanced", hint: "Default speed/reliability trade-off." },
  { value: "thorough", label: "Thorough", hint: "Tests more endpoints." },
  { value: "stealth", label: "Stealth", hint: "Low and slow scanning." },
  { value: "ironclad", label: "Ironclad", hint: "Real tunnel test per candidate. Slowest." },
];

export const NOIZE_PROFILES = ["off", "light", "firewall", "balanced", "aggressive", "gfw"];
export const IP_VERSIONS: { value: IpVersion; label: string }[] = [
  { value: "v4", label: "IPv4" },
  { value: "v6", label: "IPv6" },
  { value: "dual", label: "Dual" },
];
export const LOG_LEVELS: { value: LogLevel; label: string }[] = [
  { value: "error", label: "Error" },
  { value: "warn", label: "Warnings" },
  { value: "info", label: "Info" },
  { value: "debug", label: "Debug" },
  { value: "trace", label: "Trace (noisy)" },
];

export const api = {
  connect: (): Promise<EngineStatus> => invoke("connect"),
  disconnect: (): Promise<EngineStatus> => invoke("disconnect"),
  getStatus: (): Promise<EngineStatus> => invoke("get_status"),
  getSettings: (): Promise<NetherSettings> => invoke("get_settings"),
  saveSettings: (settings: NetherSettings): Promise<NetherSettings> =>
    invoke("save_settings", { settings }),
  recentLogs: (limit?: number): Promise<LogRecord[]> => invoke("recent_logs", { limit }),
  clearLogs: (): Promise<void> => invoke("clear_logs"),
  appInfo: (): Promise<{ name: string; version: string }> => invoke("app_info"),
  copyText: (text: string): Promise<void> =>
    import("@tauri-apps/plugin-clipboard-manager").then((m) => m.writeText(text)),
};

export function onLog(cb: (record: LogRecord) => void): Promise<() => void> {
  return listen<LogRecord>("nether://log", (e) => cb(e.payload));
}

export function onStatus(cb: (status: EngineStatus) => void): Promise<() => void> {
  return listen<EngineStatus>("nether://status", (e) => cb(e.payload));
}
