import { useEffect, useState } from "react";
import { Alert, Badge, Code, CopyButton, Group, Loader, Paper, Stack, Text } from "@mantine/core";
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

function PowerIcon({ size = 44 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.1}
      strokeLinecap="round"
    >
      <path d="M12 2.5v9" />
      <path d="M17.6 5.6a8.5 8.5 0 1 1-11.2 0" />
    </svg>
  );
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
                {copied ? "Copied" : "Copy"}
              </Text>
            )}
          </CopyButton>
        </Group>
      </Paper>
    );
  }

  return (
    <Stack align="center" gap="lg" pt="xl" pb="xl" px="md" maw={640} mx="auto">
      <Stack align="center" gap="md" pt="sm">
        <button
          className={`power-btn ${status.state}`}
          onClick={onToggle}
          disabled={busy}
          aria-label={connected ? "Disconnect" : "Connect"}
        >
          {busy ? <Loader size={38} type="dots" color="currentColor" /> : <PowerIcon />}
        </button>

        <Group gap="xs">
          <Badge size="lg" variant="light" color={BADGE_COLOR[status.state]} tt="capitalize">
            {label}
          </Badge>
          {uptime && (
            <Text size="sm" c="dimmed" ff="monospace">{uptime}</Text>
          )}
        </Group>
      </Stack>

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
          ff="monospace"
          fz="sm"
        >
          {status.detail}
        </Alert>
      )}

      <Text size="sm" c="dimmed" ta="center" maw={420}>
        {connected || busy ? (
          <>Route your apps through the proxy above. Tune the connection in{" "}
            <Text component="a" href="#" c="grape.3" onClick={(e) => { e.preventDefault(); onOpenSettings(); }} style={{ textDecoration: "none" }}>
              Settings
            </Text>
            .
          </>
        ) : (
          <>Tap the power button to open the tunnel, then route your apps through the proxy above.</>
        )}
      </Text>
    </Stack>
  );
}
