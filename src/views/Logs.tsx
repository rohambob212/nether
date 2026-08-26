import { useEffect, useRef, useState } from "react";
import { ActionIcon, Badge, SegmentedControl, Switch, Tooltip } from "@mantine/core";
import { api, onLog, type LogRecord } from "../api";

const LEVELS = ["all", "error", "warn", "info", "debug", "trace"];
const RENDER_CAP = 800;

const LEVEL_COLOR: Record<string, string> = {
  error: "red",
  warn: "yellow",
  info: "grape",
  debug: "cyan",
  trace: "gray",
};

export default function Logs({ active }: { active: boolean }) {
  const [filter, setFilter] = useState("all");
  const [autoscroll, setAutoscroll] = useState(true);
  const buffer = useRef<LogRecord[]>([]);
  const listEl = useRef<HTMLDivElement>(null);
  const [, force] = useState(0);

  useEffect(() => {
    api.recentLogs().then((hist) => {
      buffer.current = hist;
      force((n) => n + 1);
    });
    let unlisten: (() => void) | undefined;
    onLog((rec) => {
      buffer.current.push(rec);
      if (buffer.current.length > 5000) buffer.current.splice(0, 1000);
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  }, []);

  // Only re-render at 4 Hz when this tab is actually visible.
  useEffect(() => {
    if (!active) return;
    const id = setInterval(() => force((n) => n + 1), 250);
    return () => clearInterval(id);
  }, [active]);

  const shown = (
    filter === "all" ? buffer.current : buffer.current.filter((r) => r.level === filter)
  ).slice(-RENDER_CAP);

  useEffect(() => {
    if (autoscroll && listEl.current) {
      listEl.current.scrollTop = listEl.current.scrollHeight;
    }
  }, [shown.length, autoscroll]);

  function ts(ms: number): string {
    const d = new Date(ms);
    return d.toTimeString().slice(0, 8) + "." + String(d.getMilliseconds()).padStart(3, "0");
  }

  return (
    <div className="logs">
      <div className="logs-toolbar">
        <SegmentedControl
          size="xs"
          data={LEVELS.map((l) => ({ label: l.toUpperCase(), value: l }))}
          value={filter}
          onChange={(v) => setFilter(v)}
        />
        <span className="spacer" />
        <Tooltip label="Auto-scroll">
          <Switch
            size="sm"
            checked={autoscroll}
            onChange={(e) => setAutoscroll(e.currentTarget.checked)}
          />
        </Tooltip>
        <ActionIcon
          variant="default"
          size="lg"
          aria-label="Copy all"
          onClick={() => api.copyText(buffer.current.map(fmt).join("\n")).catch(() => {})}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
        </ActionIcon>
        <ActionIcon
          variant="light"
          color="red"
          size="lg"
          aria-label="Clear"
          onClick={() => {
            api.clearLogs().catch(() => {});
            buffer.current = [];
            force((n) => n + 1);
          }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
        </ActionIcon>
      </div>

      <div className="log-list mono" ref={listEl}>
        {shown.map((r, i) => (
          <div key={i} className="log-line">
            <span className="log-ts">{ts(r.tsMs)}</span>
            <Badge
              size="xs"
              variant="light"
              color={LEVEL_COLOR[r.level] ?? "gray"}
              className="log-level-badge"
            >
              {r.level.toUpperCase()}
            </Badge>
            <span className="log-msg"> {r.message}</span>
          </div>
        ))}
        {shown.length === 0 && <div className="log-empty">no log lines</div>}
      </div>
    </div>
  );
}

function fmt(r: LogRecord): string {
  return `${new Date(r.tsMs).toISOString()} [${r.level.toUpperCase()}] ${r.target}: ${r.message}`;
}
