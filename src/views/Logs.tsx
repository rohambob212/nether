import { useEffect, useRef, useState } from "react";
import { Badge, Button, Select, Switch } from "@mantine/core";
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

export default function Logs() {
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
    // Batch re-renders so a burst of log lines doesn't jank the UI.
    const id = setInterval(() => force((n) => n + 1), 250);
    return () => {
      clearInterval(id);
      unlisten?.();
    };
  }, []);

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
        <Select
          w={120}
          data={LEVELS.map((l) => ({ label: l.toUpperCase(), value: l }))}
          value={filter}
          onChange={(v) => v && setFilter(v)}
        />
        <Switch
          size="sm"
          label="Auto-scroll"
          labelPosition="left"
          checked={autoscroll}
          onChange={(e) => setAutoscroll(e.currentTarget.checked)}
        />
        <span className="spacer" />
        <Button
          size="compact-sm"
          variant="default"
          onClick={() => api.copyText(buffer.current.map(fmt).join("\n")).catch(() => {})}
        >
          Copy all
        </Button>
        <Button
          size="compact-sm"
          variant="light"
          color="red"
          onClick={() => {
            api.clearLogs().catch(() => {});
            buffer.current = [];
            force((n) => n + 1);
          }}
        >
          Clear
        </Button>
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
