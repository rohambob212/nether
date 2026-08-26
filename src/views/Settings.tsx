import { useEffect, useState, type ReactNode } from "react";
import {
  api,
  DEFAULT_SETTINGS,
  IP_VERSIONS,
  LOG_LEVELS,
  NOIZE_PROFILES,
  PROTOCOLS,
  SCAN_MODES,
  type NetherSettings,
} from "../api";

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="row">
      <div className="row-label">
        <span>{label}</span>
        {hint && <small>{hint}</small>}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}

export default function Settings() {
  const [draft, setDraft] = useState<NetherSettings | null>(null);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    api.getSettings().then(setDraft).catch(() => setDraft(DEFAULT_SETTINGS));
  }, []);

  if (!draft) return <div className="settings-loading">loading…</div>;
  const d = draft;

  function set<K extends keyof NetherSettings>(key: K, value: NetherSettings[K]) {
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
    setSaved(false);
  }

  // Text inputs bind as strings; empty becomes null for optional fields.
  function opt(key: keyof NetherSettings) {
    return {
      value: (d[key] as string | null) ?? "",
      onChange: (e: React.ChangeEvent<HTMLInputElement>) => {
        const v = e.target.value.trim();
        set(key, v === "" ? null : v);
      },
    };
  }

  function num(key: keyof NetherSettings) {
    return {
      value: (d[key] as number | null) ?? "",
      onChange: (e: React.ChangeEvent<HTMLInputElement>) => {
        const v = e.target.value;
        set(key, v === "" ? null : Math.max(0, Math.floor(Number(v) || 0)));
      },
    };
  }

  async function save() {
    try {
      const normalized = await api.saveSettings(draft!);
      setDraft(normalized);
      setSaved(true);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="settings">
      <section>
        <h2>Connection</h2>
        <Row label="Protocol">
          <div className="card-row">
            {PROTOCOLS.map((p) => (
              <button
                key={p.value}
                title={p.hint}
                className={`card ${d.protocol === p.value ? "selected" : ""}`}
                onClick={() => set("protocol", p.value)}
              >
                {p.label}
              </button>
            ))}
          </div>
        </Row>
        <Row label="Scan mode" hint={SCAN_MODES.find((s) => s.value === d.scanMode)?.hint}>
          <select value={d.scanMode} onChange={(e) => set("scanMode", e.target.value as never)}>
            {SCAN_MODES.map((s) => (
              <option key={s.value} value={s.value}>{s.label}</option>
            ))}
          </select>
        </Row>
        <Row label="IP version">
          <select value={d.ipVersion} onChange={(e) => set("ipVersion", e.target.value as never)}>
            {IP_VERSIONS.map((v) => (
              <option key={v.value} value={v.value}>{v.label}</option>
            ))}
          </select>
        </Row>
        <Row label="Quick reconnect" hint="Reuse last good gateway when possible">
          <input type="checkbox" checked={d.quickReconnect} onChange={(e) => set("quickReconnect", e.target.checked)} />
        </Row>
        <Row label="Auto-connect" hint="Connect on app launch">
          <input type="checkbox" checked={d.autoConnect} onChange={(e) => set("autoConnect", e.target.checked)} />
        </Row>
        <Row
          label="Always-on core"
          hint="Keep the tunnel established — START/STOP only toggles the proxy port"
        >
          <div className="card-row">
            <button className={`card ${!d.alwaysOn ? "selected" : ""}`} onClick={() => set("alwaysOn", false)}>
              Standard
            </button>
            <button className={`card ${d.alwaysOn ? "selected" : ""}`} onClick={() => set("alwaysOn", true)}>
              Always-on
            </button>
          </div>
        </Row>
      </section>

      <section>
        <h2>Local proxy</h2>
        <Row label="SOCKS5 bind address">
          <input
            type="text"
            className="mono"
            value={d.socksHost}
            onChange={(e) => set("socksHost", e.target.value)}
          />
          <input
            type="number"
            min={1}
            max={65535}
            value={d.socksPort}
            onChange={(e) => set("socksPort", Math.max(1, Math.min(65535, Number(e.target.value) || 1)))}
          />
        </Row>
        <Row label="HTTP proxy" hint="Also expose an HTTP CONNECT proxy">
          <input type="checkbox" checked={d.httpProxyEnabled} onChange={(e) => set("httpProxyEnabled", e.target.checked)} />
          <input
            type="number"
            min={1}
            max={65535}
            disabled={!d.httpProxyEnabled}
            value={d.httpProxyPort}
            onChange={(e) => set("httpProxyPort", Math.max(1, Math.min(65535, Number(e.target.value) || 1)))}
          />
        </Row>
        <Row label="Upstream proxy" hint="Dial out through another proxy (URL)">
          <input type="text" {...opt("upstreamProxy")} placeholder="socks5://user:pass@host:port" />
        </Row>
      </section>

      <section>
        <h2>Smart routing</h2>
        <Row label="Proxy mode">
          <div className="card-row">
            <button
              title="Plain Aether SOCKS5, like other Aether clients"
              className={`card ${!d.smartRouting ? "selected" : ""}`}
              onClick={() => set("smartRouting", false)}
            >
              Direct proxy
            </button>
            <button
              title="Route through Xray with Iran split rules and ad blocking"
              className={`card ${d.smartRouting ? "selected" : ""}`}
              onClick={() => set("smartRouting", true)}
            >
              Smart routing (Xray)
            </button>
          </div>
        </Row>
        {d.smartRouting && (
          <>
            <Row label="Master SOCKS5 port" hint="Point your apps here when smart routing is on">
              <input
                type="number"
                min={1}
                max={65535}
                value={d.xraySocksPort}
                onChange={(e) => set("xraySocksPort", Math.max(1, Math.min(65535, Number(e.target.value) || 1)))}
              />
            </Row>
            <p className="hint">
              Iranian sites and private IPs go direct, ads are blocked, everything else is
              tunneled — like Hiddify's rules.
            </p>
          </>
        )}
      </section>

      <details className="advanced">
        <summary>Advanced settings</summary>

        <section>
          <h2>Obfuscation</h2>
          <Row label="Noize profile">
            <select value={d.noizeProfile} onChange={(e) => set("noizeProfile", e.target.value)}>
              {NOIZE_PROFILES.map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
          </Row>
        </section>

        <section>
          <h2>MASQUE</h2>
          <Row label="HTTP/2 fallback">
            <input type="checkbox" checked={d.masqueH2} onChange={(e) => set("masqueH2", e.target.checked)} />
          </Row>
          <Row label="ECH config" hint="Empty = auto">
            <input type="text" {...opt("ech")} placeholder="auto" />
          </Row>
          <Row label="TLS fragmentation">
            <input type="checkbox" checked={d.fragment} onChange={(e) => set("fragment", e.target.checked)} />
          </Row>
          {d.fragment && (
            <>
              <Row label="Fragment size">
                <input type="text" value={d.fragmentSize} onChange={(e) => set("fragmentSize", e.target.value)} placeholder="e.g. 32,64" />
              </Row>
              <Row label="Fragment delay (ms)">
                <input type="text" value={d.fragmentDelay} onChange={(e) => set("fragmentDelay", e.target.value)} placeholder="e.g. 5" />
              </Row>
            </>
          )}
          <Row label="Skip data check">
            <input type="checkbox" checked={d.disableDataCheck} onChange={(e) => set("disableDataCheck", e.target.checked)} />
          </Row>
        </section>

        <section>
          <h2>WireGuard</h2>
          <Row label="Keepalive (secs)">
            <input type="number" min={0} {...num("keepaliveSecs")} />
          </Row>
          <Row label="No profile retry">
            <input type="checkbox" checked={d.noProfileRetry} onChange={(e) => set("noProfileRetry", e.target.checked)} />
          </Row>
          <Row label="Forced peer">
            <input type="text" {...opt("forcedPeer")} />
          </Row>
          <Row label="Forced WG peer">
            <input type="text" {...opt("forcedWgPeer")} />
          </Row>
        </section>

        <section>
          <h2>Cloudflare Zero Trust</h2>
          <Row label="Team name">
            <input type="text" {...opt("teamName")} />
          </Row>
          <Row label="Access client ID">
            <input type="text" {...opt("accessClientId")} />
          </Row>
          <Row label="Access client secret">
            <input type="password" {...opt("accessClientSecret")} />
          </Row>
          <Row label="Access token">
            <input type="password" {...opt("accessToken")} />
          </Row>
          <Row label="Use gateway proxy">
            <input type="checkbox" checked={d.useGatewayProxy} onChange={(e) => set("useGatewayProxy", e.target.checked)} />
          </Row>
        </section>

        <section>
          <h2>Network tuning</h2>
          <Row label="DNS resolvers">
            <input type="text" value={d.dnsResolvers} onChange={(e) => set("dnsResolvers", e.target.value)} />
          </Row>
          <Row label="Validate timeout (secs)">
            <input type="number" min={0} {...num("validateSecs")} />
          </Row>
          <Row label="Startup timeout (secs)">
            <input type="number" min={0} {...num("startupSecs")} />
          </Row>
          <Row label="Reconnect delay (secs)">
            <input type="number" min={0} {...num("reconnectSecs")} />
          </Row>
          <Row label="Route block" hint="CIDRs to send through tunnel">
            <input type="text" {...opt("routeBlock")} />
          </Row>
          <Row label="Route direct" hint="CIDRs to bypass tunnel">
            <input type="text" {...opt("routeDirect")} />
          </Row>
          <Row label="Perf profile">
            <input type="text" {...opt("perfProfile")} />
          </Row>
          <Row label="TLS groups">
            <input type="text" {...opt("tlsGroups")} />
          </Row>
        </section>

        <section>
          <h2>App</h2>
          <Row label="Log level">
            <select value={d.logLevel} onChange={(e) => set("logLevel", e.target.value as never)}>
              {LOG_LEVELS.map((l) => (
                <option key={l.value} value={l.value}>{l.label}</option>
              ))}
            </select>
          </Row>
        </section>
      </details>

      <footer className="settings-footer">
        <button
          className="primary"
          onClick={() => api.getSettings().then(setDraft).catch(() => {}).then(() => setSaved(false))}
        >
          RESET
        </button>
        <span className="spacer" />
        {error && <span className="err">{error}</span>}
        {saved && !error && <span className="ok">saved ✓</span>}
        <button className="primary" onClick={save}>
          SAVE
        </button>
      </footer>
    </div>
  );
}
