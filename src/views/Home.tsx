import { useEffect, useState } from "react";
import { Alert, Badge, Code, CopyButton, Group, Paper, Stack, Text } from "@mantine/core";
import { api, type EngineStatus, type NetherSettings } from "../api";

function formatUptime(sinceMs: number): string {
  const s = Math.floor((Date.now() - sinceMs) / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : m > 0 ? `${m}m ${s % 60}s` : `${s}s`;
}

const BADGE_COLOR: Record<string, string> = {
  connected: "green",
  connecting: "grape",
  disconnecting: "grape",
  disconnected: "gray",
};

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
    status.state === "disconnected" ? "Offline"
    : status.state === "connecting" ? status.phase ?? "Connecting…"
    : status.state === "disconnecting" ? "Disconnecting…"
    : "Connected";

  const errored = !!status.detail && /error|failed|\[-\]/i.test(status.detail);
  const smart = settings?.smartRouting ?? false;
  const tunnelAddr = settings ? `${settings.socksHost}:${settings.socksPort}` : null;
  const masterAddr = settings && smart ? `${settings.socksHost}:${settings.xraySocksPort}` : null;

  function AddrCard({ labelText, addr }: { labelText: string; addr: string }) {
    return (
      <Paper withBorder radius="md" p="md" bg="dark.7">
        <Text size="xs" fw={700} c="dimmed" tt="uppercase" lh={1.4} mb={6}>
          {labelText}
        </Text>
        <Group gap="xs" wrap="nowrap">
          <Code flex={1} style={{ overflowWrap: "anywhere" }}>{addr}</Code>
          <CopyButton value={addr}>
            {({ copied, copy }) => (
              <Text
                component="button"
                c={copied ? "green.4" : "grape.3"}
                size="xs"
                fw={700}
                onClick={copy}
                style={{ background: "none", border: "none", cursor: "pointer", flexShrink: 0 }}
              >
                {copied ? "COPIED" : "COPY"}
              </Text>
            )}
          </CopyButton>
        </Group>
      </Paper>
    );
  }

  return (
    <Stack align="center" gap="lg" pt="xl" pb="xl" px="md" maw={640} mx="auto">
      <button className={`portal-btn ${status.state}`} onClick={onToggle} disabled={busy}>
        <img src="/portal.png" alt="" />
        <span className="portal-label">
          {connected ? "DISCONNECT" : busy ? "…" : "CONNECT"}
        </span>
      </button>

      <Group gap="sm">
        <Badge size="lg" variant="light" color={BADGE_COLOR[status.state]} tt="capitalize">
          {label}
        </Badge>
        {uptime && (
          <Text size="sm" c="dimmed" ff="monospace">{uptime}</Text>
        )}
      </Group>

      {(masterAddr ?? tunnelAddr) && (
        <Stack gap="sm" w="100%">
          {masterAddr && <AddrCard labelText="Proxy — smart routing" addr={masterAddr} />}
          {tunnelAddr && (
            <AddrCard labelText={smart ? "Raw tunnel" : "SOCKS5 proxy"} addr={tunnelAddr} />
          )}
          {status.gateway && (
            <Paper withBorder radius="md" p="md" bg="dark.7">
              <Text size="xs" fw={700} c="dimmed" tt="uppercase" lh={1.4} mb={6}>
                Gateway
              </Text>
              <Code style={{ overflowWrap: "anywhere" }}>{status.gateway}</Code>
            </Paper>
          )}
        </Stack>
      )}

      {status.detail && (
        <Alert
          w="100%"
          variant="light"
          color={errored ? "red" : "grape"}
          icon={errored ? "✕" : "ℹ"}
          ff="monospace"
          fz="sm"
        >
          {status.detail}
        </Alert>
      )}

      <Text size="sm" c="dimmed" ta="center" maw={420}>
        Route your apps through the SOCKS5 proxy above. Tune the connection in{" "}
        <Text component="a" href="#" c="grape.3" onClick={(e) => { e.preventDefault(); onOpenSettings(); }} style={{ textDecoration: "none" }}>
          Settings
        </Text>
        .
      </Text>
    </Stack>
  );
}
