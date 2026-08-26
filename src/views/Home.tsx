import { useEffect, useState } from "react";
import { api, type EngineStatus, type NetherSettings } from "../api";

function formatUptime(sinceMs: number): string {
  const s = Math.floor((Date.now() - sinceMs) / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : m > 0 ? `${m}m ${s % 60}s` : `${s}s`;
}

export default function Home({
  status,
  busy,
  onToggle,
  onOpenSettings,
}: {
  status: EngineStatus;
  busy: boolean;
  onToggle: () => void;
  onOpenSettings: () => void;
}) {
  const connected = status.state === "connected";
  const [uptime, setUptime] = useState("");
  const [settings, setSettings] = useState<NetherSettings | null>(null);

  useEffect(() => {
    api.getSettings().then(setSettings).catch(() => {});
  }, []);

  useEffect(() => {
    if (!connected || !status.connectedSinceMs) return;
    const tick = () => setUptime(formatUptime(status.connectedSinceMs!));
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [connected, status.connectedSinceMs]);

  const label =
    status.state === "disconnected" ? "CONNECT"
    : status.state === "connecting" ? (status.phase ?? "CONNECTING").toUpperCase()
    : status.state === "disconnecting" ? "DISCONNECTING"
    : "DISCONNECT";

  const errored = !!status.detail && /error|failed|\[-\]/i.test(status.detail);
  const smart = settings?.smartRouting ?? false;
  const tunnelAddr = settings ? `${settings.socksHost}:${settings.socksPort}` : null;
  const masterAddr = settings && smart ? `${settings.socksHost}:${settings.xraySocksPort}` : null;

  return (
    <div className="home">
      <button
        className={`portal-ring ${status.state}`}
        onClick={onToggle}
        disabled={busy}
      >
        <span className="ring-inner">{connected ? "\u25A0 STOP" : "\u25B6 START"}</span>
      </button>

      <div className="state-line">
        <span className={`state-text ${status.state}`}>{label}</span>
        {uptime && <span className="uptime">{uptime}</span>}
      </div>

      {connected && (masterAddr ?? tunnelAddr) && (
        <div className="info-grid">
          {masterAddr && (
            <div className="info-card">
              <div className="info-label">PROXY — SMART ROUTING</div>
              <div className="info-value mono">
                {masterAddr}
                <button className="mini-btn" onClick={() => api.copyText(masterAddr).catch(() => {})}>
                  COPY
                </button>
              </div>
            </div>
          )}
          {tunnelAddr && (
            <div className="info-card">
              <div className="info-label">{smart ? "RAW TUNNEL" : "SOCKS5 PROXY"}</div>
              <div className="info-value mono">
                {tunnelAddr}
                <button className="mini-btn" onClick={() => api.copyText(tunnelAddr).catch(() => {})}>
                  COPY
                </button>
              </div>
            </div>
          )}
          {status.gateway && (
            <div className="info-card">
              <div className="info-label">GATEWAY</div>
              <div className="info-value mono">{status.gateway}</div>
            </div>
          )}
        </div>
      )}

      {status.detail && <p className={`detail ${errored ? "err" : ""}`}>{status.detail}</p>}

      <p className="hint">
        Route your apps through the SOCKS5 proxy above. Tune the connection in{" "}
        <a href="#" onClick={(e) => { e.preventDefault(); onOpenSettings(); }}>
          Settings
        </a>
        .
      </p>
    </div>
  );
}
