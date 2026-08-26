import { useEffect, useRef, useState } from "react";
import { api, onLog, type LogRecord } from "../api";

const LEVELS = ["all", "error", "warn", "info", "debug", "trace"];
const RENDER_CAP = 800;

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
        <select value={filter} onChange={(e) => setFilter(e.target.value)}>
          {LEVELS.map((l) => (
            <option key={l} value={l}>
              {l.toUpperCase()}
            </option>
          ))}
        </select>
        <label className="toggle">
          <input
            type="checkbox"
            checked={autoscroll}
            onChange={(e) => setAutoscroll(e.target.checked)}
          />
          auto-scroll
        </label>
        <span className="spacer" />
        <button className="mini-btn" onClick={() => api.copyText(buffer.current.map(fmt).join("\n")).catch(() => {})}>
          COPY ALL
        </button>
        <button
          className="mini-btn"
          onClick={() => {
            api.clearLogs().catch(() => {});
            buffer.current = [];
            force((n) => n + 1);
          }}
        >
          CLEAR
        </button>
      </div>

      <div className="log-list mono" ref={listEl}>
        {shown.map((r, i) => (
          <div key={i} className={`log-line level-${r.level}`}>
            <span className="log-ts">{ts(r.tsMs)}</span>
            <span className="log-level">{r.level.toUpperCase().padEnd(5)}</span>
            <span className="log-msg">{r.message}</span>
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
