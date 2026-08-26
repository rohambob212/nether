import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Group,
  NumberInput,
  Paper,
  PasswordInput,
  SegmentedControl,
  Select,
  Stack,
  Switch,
  Text,
  TextInput,
} from "@mantine/core";
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

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Paper withBorder radius="md" p="md" bg="dark.7">
      <Text size="xs" fw={700} c="grape.3" tt="uppercase" mb="xs">
        {title}
      </Text>
      <Stack gap="sm">{children}</Stack>
    </Paper>
  );
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <Group justify="space-between" align="center" gap="md" wrap="wrap">
      <Stack gap={2} style={{ flex: "1 1 200px" }}>
        <Text size="sm" fw={500}>{label}</Text>
        {hint && <Text size="xs" c="dimmed">{hint}</Text>}
      </Stack>
      <Group gap="sm" style={{ flexShrink: 0 }}>{children}</Group>
    </Group>
  );
}

export default function Settings() {
  const [draft, setDraft] = useState<NetherSettings | null>(null);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [error, setError] = useState("");
  const savedJson = useRef<string | null>(null);
  const loaded = useRef(false);

  useEffect(() => {
    api.getSettings().then((s) => {
      savedJson.current = JSON.stringify(s);
      setDraft(s);
      loaded.current = true;
    }).catch(() => setDraft(DEFAULT_SETTINGS));
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

  if (!draft) return <Text c="dimmed" ta="center" pt="xl">loading…</Text>;
  const d = draft;

  function set<K extends keyof NetherSettings>(key: K, value: NetherSettings[K]) {
    setDraft((prev) => (prev ? { ...prev, [key]: value } : prev));
  }

  // Text inputs bind as strings; empty becomes null for optional fields.
  function opt(key: keyof NetherSettings) {
    return {
      value: (d[key] as string | null) ?? "",
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
      // Ignore the transient empty value so clearing the field to retype a port
      // doesn't snap it to 1 (and auto-save that).
      onChange: (v: string | number) => v !== "" && set(key, Number(v)),
      w: 110,
    };
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

  return (
    <Stack gap="md" maw={680} mx="auto" p="md" pb={48}>
      <Group justify="center" gap="xs" mih={22}>
        {saveState === "saving" && <Badge size="sm" color="grape" variant="light">saving…</Badge>}
        {saveState === "saved" && <Badge size="sm" color="green" variant="light">saved</Badge>}
        {saveState === "error" && <Badge size="sm" color="red" variant="light">save failed</Badge>}
      </Group>
      {saveState === "error" && (
        <Alert variant="light" color="red" fz="sm">{error}</Alert>
      )}

      <Section title="Connection">
        <Row label="Protocol">
          <SegmentedControl
            data={PROTOCOLS.map((p) => ({ label: p.label, value: p.value }))}
            value={d.protocol}
            onChange={(v) => set("protocol", v as never)}
          />
        </Row>
        <Row label="Scan mode" hint={SCAN_MODES.find((s) => s.value === d.scanMode)?.hint}>
          <Select
            w={180}
            data={SCAN_MODES.map((s) => ({ label: s.label, value: s.value }))}
            value={d.scanMode}
            onChange={(v) => v && set("scanMode", v as never)}
          />
        </Row>
        <Row label="IP version">
          <Select
            w={180}
            data={IP_VERSIONS.map((v) => ({ label: v.label, value: v.value }))}
            value={d.ipVersion}
            onChange={(v) => v && set("ipVersion", v as never)}
          />
        </Row>
        <Row label="Quick reconnect" hint="Reuse last good gateway when possible">
          <Switch checked={d.quickReconnect} onChange={(e) => set("quickReconnect", e.currentTarget.checked)} />
        </Row>
        <Row label="Auto-connect" hint="Connect on app launch">
          <Switch checked={d.autoConnect} onChange={(e) => set("autoConnect", e.currentTarget.checked)} />
        </Row>
        <Row
          label="Always-on core"
          hint="Keep the tunnel established — START/STOP only toggles the proxy port"
        >
          <Switch checked={d.alwaysOn} onChange={(e) => set("alwaysOn", e.currentTarget.checked)} />
        </Row>
      </Section>

      <Section title="Local proxy">
        <Row label="SOCKS5 bind address">
          <Group gap="xs">
            <TextInput w={140} className="mono" value={d.socksHost} onChange={(e) => set("socksHost", e.currentTarget.value)} />
            <NumberInput {...port("socksPort")} hideControls />
          </Group>
        </Row>
        <Row label="HTTP proxy" hint="Also expose an HTTP CONNECT proxy">
          <Group gap="sm">
            <Switch checked={d.httpProxyEnabled} onChange={(e) => set("httpProxyEnabled", e.currentTarget.checked)} />
            <NumberInput {...port("httpProxyPort")} hideControls disabled={!d.httpProxyEnabled} />
          </Group>
        </Row>
        <Row label="Upstream proxy" hint="Dial out through another proxy (URL)">
          <TextInput w={260} placeholder="socks5://user:pass@host:port" {...opt("upstreamProxy")} />
        </Row>
      </Section>

      <Section title="Smart routing">
        <Row label="Proxy mode">
          <SegmentedControl
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
            <Row label="Master SOCKS5 port" hint="Point your apps here when smart routing is on">
              <NumberInput {...port("xraySocksPort")} hideControls />
            </Row>
            <Text size="xs" c="dimmed">
              Iranian sites and private IPs go direct, ads are blocked, everything else is
              tunneled — like Hiddify's rules.
            </Text>
          </>
        )}
      </Section>

      <details className="advanced">
        <summary>Advanced settings</summary>

        <Stack gap="md" pt="md">
          <Section title="Obfuscation">
            <Row label="Noize profile">
              <Select
                w={180}
                data={NOIZE_PROFILES.map((n) => ({ label: n, value: n }))}
                value={d.noizeProfile}
                onChange={(v) => v && set("noizeProfile", v)}
              />
            </Row>
          </Section>

          <Section title="MASQUE">
            <Row label="HTTP/2 fallback">
              <Switch checked={d.masqueH2} onChange={(e) => set("masqueH2", e.currentTarget.checked)} />
            </Row>
            <Row label="ECH config" hint="Empty = auto">
              <TextInput w={260} placeholder="auto" {...opt("ech")} />
            </Row>
            <Row label="TLS fragmentation">
              <Switch checked={d.fragment} onChange={(e) => set("fragment", e.currentTarget.checked)} />
            </Row>
            {d.fragment && (
              <>
                <Row label="Fragment size">
                  <TextInput w={260} value={d.fragmentSize} onChange={(e) => set("fragmentSize", e.currentTarget.value)} placeholder="e.g. 32,64" />
                </Row>
                <Row label="Fragment delay (ms)">
                  <TextInput w={260} value={d.fragmentDelay} onChange={(e) => set("fragmentDelay", e.currentTarget.value)} placeholder="e.g. 5" />
                </Row>
              </>
            )}
            <Row label="Skip data check">
              <Switch checked={d.disableDataCheck} onChange={(e) => set("disableDataCheck", e.currentTarget.checked)} />
            </Row>
          </Section>

          <Section title="WireGuard">
            <Row label="Keepalive (secs)">
              <NumberInput {...num("keepaliveSecs")} w={110} hideControls />
            </Row>
            <Row label="No profile retry">
              <Switch checked={d.noProfileRetry} onChange={(e) => set("noProfileRetry", e.currentTarget.checked)} />
            </Row>
            <Row label="Forced peer">
              <TextInput w={260} {...opt("forcedPeer")} />
            </Row>
            <Row label="Forced WG peer">
              <TextInput w={260} {...opt("forcedWgPeer")} />
            </Row>
          </Section>

          <Section title="Cloudflare Zero Trust">
            <Row label="Team name">
              <TextInput w={260} {...opt("teamName")} />
            </Row>
            <Row label="Access client ID">
              <TextInput w={260} {...opt("accessClientId")} />
            </Row>
            <Row label="Access client secret">
              <PasswordInput w={260} {...opt("accessClientSecret")} />
            </Row>
            <Row label="Access token">
              <PasswordInput w={260} {...opt("accessToken")} />
            </Row>
            <Row label="Use gateway proxy">
              <Switch checked={d.useGatewayProxy} onChange={(e) => set("useGatewayProxy", e.currentTarget.checked)} />
            </Row>
          </Section>

          <Section title="Network tuning">
            <Row label="DNS resolvers">
              <TextInput w={260} value={d.dnsResolvers} onChange={(e) => set("dnsResolvers", e.currentTarget.value)} />
            </Row>
            <Row label="Validate timeout (secs)">
              <NumberInput {...num("validateSecs")} w={110} hideControls />
            </Row>
            <Row label="Startup timeout (secs)">
              <NumberInput {...num("startupSecs")} w={110} hideControls />
            </Row>
            <Row label="Reconnect delay (secs)">
              <NumberInput {...num("reconnectSecs")} w={110} hideControls />
            </Row>
            <Row label="Route block" hint="CIDRs to send through tunnel">
              <TextInput w={260} {...opt("routeBlock")} />
            </Row>
            <Row label="Route direct" hint="CIDRs to bypass tunnel">
              <TextInput w={260} {...opt("routeDirect")} />
            </Row>
            <Row label="Perf profile">
              <TextInput w={260} {...opt("perfProfile")} />
            </Row>
            <Row label="TLS groups">
              <TextInput w={260} {...opt("tlsGroups")} />
            </Row>
          </Section>

          <Section title="App">
            <Row label="Log level">
              <Select
                w={180}
                data={LOG_LEVELS.map((l) => ({ label: l.label, value: l.value }))}
                value={d.logLevel}
                onChange={(v) => v && set("logLevel", v as never)}
              />
            </Row>
          </Section>

          <Button variant="subtle" color="red" onClick={reset}>
            Reset all settings to defaults
          </Button>
        </Stack>
      </details>
    </Stack>
  );
}
