import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  Alert,
  NumberInput,
  PasswordInput,
  SegmentedControl,
  Select,
  Switch,
  TextInput,
} from "@mantine/core";
import {
  api,
  DEFAULT_SETTINGS,
  IP_VERSIONS,
  IS_ANDROID,
  LOG_LEVELS,
  NOIZE_PROFILES,
  PROTOCOLS,
  SCAN_MODES,
  type NetherSettings,
} from "../api";

// Inline so the icons ship with the bundle instead of pulling an icon package
// in for nine glyphs.
const icon = (paths: ReactNode) => (
  <svg
    width="15"
    height="15"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    {paths}
  </svg>
);

const ICON = {
  shield: icon(<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />),
  link: icon(
    <>
      <path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7" />
      <path d="M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7" />
    </>
  ),
  plug: icon(
    <>
      <path d="M9 2v6M15 2v6" />
      <path d="M6 8h12v3a6 6 0 0 1-12 0z" />
      <path d="M12 17v5" />
    </>
  ),
  route: icon(
    <>
      <circle cx="6" cy="19" r="3" />
      <circle cx="18" cy="5" r="3" />
      <path d="M9 19h5a4 4 0 0 0 0-8h-4a4 4 0 0 1 0-8h5" />
    </>
  ),
  mask: icon(
    <>
      <path d="M4 8s2-3 8-3 8 3 8 3v4a7 7 0 0 1-8 7 7 7 0 0 1-8-7z" />
      <path d="M9 12h.01M15 12h.01" />
    </>
  ),
  globe: icon(
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18z" />
    </>
  ),
  key: icon(
    <>
      <circle cx="7.5" cy="15.5" r="3.5" />
      <path d="M10 13l9-9M17 6l2 2M14 9l2 2" />
    </>
  ),
  sliders: icon(
    <>
      <path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3" />
      <path d="M1 14h6M9 8h6M17 16h6" />
    </>
  ),
  app: icon(
    <>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <path d="M9 9h6v6H9z" />
    </>
  ),
};

function Card({
  title,
  glyph,
  children,
}: {
  title: string;
  glyph: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="set-card">
      <header>
        {glyph}
        {title}
      </header>
      {children}
    </section>
  );
}

/** Label on the left, control on the right; stacks on narrow screens. */
function Row({
  label,
  hint,
  wide,
  children,
}: {
  label: string;
  hint?: string;
  /** Give the control its own full-width line (segmented controls need it). */
  wide?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={wide ? "set-row wide" : "set-row"}>
      <div className="set-row-label">
        <b>{label}</b>
        {hint && <small>{hint}</small>}
      </div>
      <div className="set-row-control">{children}</div>
    </div>
  );
}

/** A switch row. Never stacks — the control is small enough to stay pinned. */
function Toggle({
  label,
  hint,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="set-row compact">
      <div className="set-row-label">
        <b>{label}</b>
        {hint && <small>{hint}</small>}
      </div>
      <div className="set-row-control">
        <Switch
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.currentTarget.checked)}
        />
      </div>
    </div>
  );
}

export default function Settings() {
  const [draft, setDraft] = useState<NetherSettings | null>(null);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [error, setError] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const savedJson = useRef<string | null>(null);
  const loaded = useRef(false);

  useEffect(() => {
    api
      .getSettings()
      .then((s) => {
        savedJson.current = JSON.stringify(s);
        setDraft(s);
        loaded.current = true;
      })
      .catch(() => setDraft(DEFAULT_SETTINGS));
  }, []);

  // Auto-save: debounce every change, no save button to babysit.
  useEffect(() => {
    if (!draft || !loaded.current) return;
    const json = JSON.stringify(draft);
    if (json === savedJson.current) return;
    setSaveState("saving");
    const id = setTimeout(async () => {
      try {
        const normalized = await api.saveSettings(draft);
        savedJson.current = JSON.stringify(normalized);
        setDraft(normalized);
        setSaveState("saved");
        setError("");
      } catch (e) {
        setSaveState("error");
        setError(String(e));
      }
    }, 600);
    return () => clearTimeout(id);
  }, [draft]);

  // Let "saved" fade out on its own; a badge that never leaves stops meaning
  // anything and just sits there.
  useEffect(() => {
    if (saveState !== "saved") return;
    const id = setTimeout(() => setSaveState("idle"), 1600);
    return () => clearTimeout(id);
  }, [saveState]);

  if (!draft) return <div className="set-note" style={{ textAlign: "center", paddingTop: 40 }}>loading…</div>;
  const d = draft;

  function set<K extends keyof NetherSettings>(key: K, value: NetherSettings[K]) {
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
  }

  // Text inputs bind as strings; empty becomes null for optional fields.
  function opt(key: keyof NetherSettings) {
    return {
      value: (d[key] as string | null) ?? "",
      w: "100%",
      // No trim here: it ate spaces as you typed. normalized() trims on save.
      onChange: (e: React.ChangeEvent<HTMLInputElement>) => {
        const v = e.target.value;
        set(key, v === "" ? null : v);
      },
    };
  }

  function num(key: keyof NetherSettings) {
    return {
      value: (d[key] as number | null) ?? "",
      w: 110,
      hideControls: true,
      onChange: (v: string | number) =>
        set(key, v === "" ? null : Math.max(0, Math.floor(Number(v) || 0))),
    };
  }

  function port(key: "socksPort" | "httpProxyPort" | "xraySocksPort") {
    return {
      value: d[key],
      min: 1,
      max: 65535,
      clampBehavior: "blur" as const,
      hideControls: true,
      // Ignore the transient empty value so clearing the field to retype a port
      // doesn't snap it to 1 (and auto-save that).
      onChange: (v: string | number) => v !== "" && set(key, Number(v)),
      w: 104,
    };
  }

  /**
   * Turning VPN mode on needs Android's consent dialog first — the switch only
   * moves if the user actually grants it.
   */
  async function toggleVpn(on: boolean) {
    if (!on) {
      set("vpnMode", false);
      return;
    }
    try {
      if (await api.vpnPrepare()) {
        set("vpnMode", true);
      } else {
        setSaveState("error");
        setError("Android declined the VPN permission, so VPN mode stays off.");
      }
    } catch (e) {
      setSaveState("error");
      setError(String(e));
    }
  }

  async function reset() {
    if (!confirm("Reset all settings to defaults?")) return;
    try {
      const normalized = await api.saveSettings(DEFAULT_SETTINGS);
      savedJson.current = JSON.stringify(normalized);
      setDraft(normalized);
      setSaveState("saved");
    } catch (e) {
      setSaveState("error");
      setError(String(e));
    }
  }

  const saveLabel =
    saveState === "saving" ? "saving…" : saveState === "saved" ? "saved" : "save failed";

  return (
    <div className="settings">
      <div className="set-save" data-state={saveState} data-show={saveState !== "idle"}>
        <span>{saveLabel}</span>
      </div>

      {saveState === "error" && (
        <Alert variant="light" color="red" fz="sm" radius="md">
          {error}
        </Alert>
      )}

      <Card title="VPN mode" glyph={ICON.shield}>
        <Toggle
          label="Route the whole device"
          hint={
            IS_ANDROID
              ? "Every app goes through the tunnel, no per-app proxy setup. Nether itself stays outside it."
              : "Needs Android's VpnService. On desktop, point apps at the SOCKS5 proxy instead."
          }
          checked={d.vpnMode}
          disabled={!IS_ANDROID}
          onChange={toggleVpn}
        />
        {d.vpnMode && (
          <div className="set-note">
            Android will show a VPN key in the status bar while the tunnel is up.
            Traffic still flows through the local proxy — VPN mode only decides
            who gets captured.
          </div>
        )}
      </Card>

      <Card title="Connection" glyph={ICON.link}>
        <Row label="Protocol" wide>
          <SegmentedControl
            fullWidth
            data={PROTOCOLS.map((p) => ({ label: p.label, value: p.value }))}
            value={d.protocol}
            onChange={(v) => set("protocol", v as never)}
          />
        </Row>
        <Row label="Scan mode" hint={SCAN_MODES.find((s) => s.value === d.scanMode)?.hint}>
          <Select
            w="100%"
            data={SCAN_MODES.map((s) => ({ label: s.label, value: s.value }))}
            value={d.scanMode}
            onChange={(v) => v && set("scanMode", v as never)}
          />
        </Row>
        <Row label="IP version">
          <Select
            w="100%"
            data={IP_VERSIONS.map((v) => ({ label: v.label, value: v.value }))}
            value={d.ipVersion}
            onChange={(v) => v && set("ipVersion", v as never)}
          />
        </Row>
        <Toggle
          label="Quick reconnect"
          hint="Reuse the last good gateway when possible"
          checked={d.quickReconnect}
          onChange={(v) => set("quickReconnect", v)}
        />
        <Toggle
          label="Auto-connect"
          hint="Connect on app launch"
          checked={d.autoConnect}
          onChange={(v) => set("autoConnect", v)}
        />
        <Toggle
          label="Always-on core"
          hint="Keep the tunnel established — the power button only toggles the proxy port"
          checked={d.alwaysOn}
          onChange={(v) => set("alwaysOn", v)}
        />
      </Card>

      <Card title="Local proxy" glyph={ICON.plug}>
        <Row label="SOCKS5 bind address">
          <TextInput
            flex={1}
            className="mono"
            value={d.socksHost}
            onChange={(e) => set("socksHost", e.currentTarget.value)}
          />
          <NumberInput {...port("socksPort")} />
        </Row>
        <Row label="HTTP proxy" hint="Also expose an HTTP CONNECT proxy">
          <Switch
            checked={d.httpProxyEnabled}
            onChange={(e) => set("httpProxyEnabled", e.currentTarget.checked)}
          />
          <NumberInput {...port("httpProxyPort")} disabled={!d.httpProxyEnabled} />
        </Row>
        <Row label="Upstream proxy" hint="Dial out through another proxy (URL)">
          <TextInput placeholder="socks5://user:pass@host:port" {...opt("upstreamProxy")} />
        </Row>
      </Card>

      <Card title="Smart routing" glyph={ICON.route}>
        <Row label="Proxy mode" wide>
          <SegmentedControl
            fullWidth
            data={[
              { label: "Direct proxy", value: "direct" },
              { label: "Smart (Xray)", value: "smart" },
            ]}
            value={d.smartRouting ? "smart" : "direct"}
            onChange={(v) => set("smartRouting", v === "smart")}
          />
        </Row>
        {d.smartRouting && (
          <>
            <Row
              label="Master SOCKS5 port"
              hint="Point your apps here when smart routing is on"
            >
              <NumberInput {...port("xraySocksPort")} />
            </Row>
            <div className="set-note">
              Iranian sites and private IPs go direct, ads are blocked, everything
              else is tunneled — like Hiddify's rules. Desktop only; the Xray
              sidecar is not bundled on Android.
            </div>
          </>
        )}
      </Card>

      <button
        className="set-disclosure"
        aria-expanded={advanced}
        onClick={() => setAdvanced((v) => !v)}
      >
        {ICON.sliders}
        Advanced settings
        <svg
          className="chev"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {advanced && (
        <div className="set-advanced">
          <Card title="Obfuscation" glyph={ICON.mask}>
            <Row label="Noize profile">
              <Select
                w="100%"
                data={NOIZE_PROFILES.map((n) => ({ label: n, value: n }))}
                value={d.noizeProfile}
                onChange={(v) => v && set("noizeProfile", v)}
              />
            </Row>
          </Card>

          <Card title="MASQUE" glyph={ICON.globe}>
            <Toggle
              label="HTTP/2 fallback"
              checked={d.masqueH2}
              onChange={(v) => set("masqueH2", v)}
            />
            <Row label="ECH config" hint="Empty = auto">
              <TextInput placeholder="auto" {...opt("ech")} />
            </Row>
            <Toggle
              label="TLS fragmentation"
              checked={d.fragment}
              onChange={(v) => set("fragment", v)}
            />
            {d.fragment && (
              <>
                <Row label="Fragment size">
                  <TextInput
                    w="100%"
                    value={d.fragmentSize}
                    placeholder="e.g. 32,64"
                    onChange={(e) => set("fragmentSize", e.currentTarget.value)}
                  />
                </Row>
                <Row label="Fragment delay (ms)">
                  <TextInput
                    w="100%"
                    value={d.fragmentDelay}
                    placeholder="e.g. 5"
                    onChange={(e) => set("fragmentDelay", e.currentTarget.value)}
                  />
                </Row>
              </>
            )}
            <Toggle
              label="Skip data check"
              checked={d.disableDataCheck}
              onChange={(v) => set("disableDataCheck", v)}
            />
          </Card>

          <Card title="WireGuard" glyph={ICON.link}>
            <Row label="Keepalive (secs)">
              <NumberInput {...num("keepaliveSecs")} />
            </Row>
            <Toggle
              label="No profile retry"
              checked={d.noProfileRetry}
              onChange={(v) => set("noProfileRetry", v)}
            />
            <Row label="Forced peer">
              <TextInput {...opt("forcedPeer")} />
            </Row>
            <Row label="Forced WG peer">
              <TextInput {...opt("forcedWgPeer")} />
            </Row>
          </Card>

          <Card title="Cloudflare Zero Trust" glyph={ICON.key}>
            <Row label="Team name">
              <TextInput {...opt("teamName")} />
            </Row>
            <Row label="Access client ID">
              <TextInput {...opt("accessClientId")} />
            </Row>
            <Row label="Access client secret">
              <PasswordInput {...opt("accessClientSecret")} />
            </Row>
            <Row label="Access token">
              <PasswordInput {...opt("accessToken")} />
            </Row>
            <Toggle
              label="Use gateway proxy"
              checked={d.useGatewayProxy}
              onChange={(v) => set("useGatewayProxy", v)}
            />
          </Card>

          <Card title="Network tuning" glyph={ICON.sliders}>
            <Row label="DNS resolvers">
              <TextInput
                w="100%"
                value={d.dnsResolvers}
                onChange={(e) => set("dnsResolvers", e.currentTarget.value)}
              />
            </Row>
            <Row label="Validate timeout (secs)">
              <NumberInput {...num("validateSecs")} />
            </Row>
            <Row label="Startup timeout (secs)">
              <NumberInput {...num("startupSecs")} />
            </Row>
            <Row label="Reconnect delay (secs)">
              <NumberInput {...num("reconnectSecs")} />
            </Row>
            <Row label="Route block" hint="CIDRs to send through tunnel">
              <TextInput {...opt("routeBlock")} />
            </Row>
            <Row label="Route direct" hint="CIDRs to bypass tunnel">
              <TextInput {...opt("routeDirect")} />
            </Row>
            <Row label="Perf profile">
              <TextInput {...opt("perfProfile")} />
            </Row>
            <Row label="TLS groups">
              <TextInput {...opt("tlsGroups")} />
            </Row>
          </Card>

          <Card title="App" glyph={ICON.app}>
            <Row label="Log level">
              <Select
                w="100%"
                data={LOG_LEVELS.map((l) => ({ label: l.label, value: l.value }))}
                value={d.logLevel}
                onChange={(v) => v && set("logLevel", v as never)}
              />
            </Row>
            <Row label="Reset" hint="Put every setting back to its default">
              <button className="set-danger" onClick={reset}>
                Reset all settings
              </button>
            </Row>
          </Card>
        </div>
      )}
    </div>
  );
}
