import { useEffect, useState, type ReactNode } from "react";
import { api, type EngineStatus } from "./api";
import { UnstyledButton } from "@mantine/core";
import Home from "./views/Home";
import Logs from "./views/Logs";
import Settings from "./views/Settings";

type Tab = "home" | "logs" | "settings";

const ICONS: Record<Tab, ReactNode> = {
  home: (
    <svg width="21" height="21" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
      <polyline points="9 22 9 12 15 12 15 22" />
    </svg>
  ),
  logs: (
    <svg width="21" height="21" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  ),
  settings: (
    <svg width="21" height="21" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <line x1="4" y1="21" x2="4" y2="14" /><line x1="4" y1="10" x2="4" y2="3" />
      <line x1="12" y1="21" x2="12" y2="12" /><line x1="12" y1="8" x2="12" y2="3" />
      <line x1="20" y1="21" x2="20" y2="16" /><line x1="20" y1="12" x2="20" y2="3" />
      <line x1="1" y1="14" x2="7" y2="14" /><line x1="9" y1="8" x2="15" y2="8" />
      <line x1="17" y1="16" x2="23" y2="16" />
    </svg>
  ),
};

const LABELS: Record<Tab, string> = { home: "Home", logs: "Logs", settings: "Settings" };

export default function App() {
  const [tab, setTab] = useState<Tab>("home");
  const [status, setStatus] = useState<EngineStatus>({
    state: "disconnected",
    phase: null,
    gateway: null,
    detail: null,
    connectedSinceMs: null,
  });
  const [version, setVersion] = useState("");

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    api.getStatus().then(setStatus).catch(() => {});
    api.appInfo().then((i) => setVersion(i.version)).catch(() => {});
    import("./api").then(({ onStatus }) =>
      onStatus(setStatus).then((u) => (unlisten = u))
    );
    return () => unlisten?.();
  }, []);

  const busy = status.state === "connecting" || status.state === "disconnecting";

  async function toggle() {
    try {
      setStatus(await (status.state === "disconnected" ? api.connect() : api.disconnect()));
    } catch (e) {
      console.error(e);
    }
  }

  // All views stay mounted so switching tabs keeps their state (settings
  // draft, log scroll) and never remount-janks. CSS animates the swap.
  return (
    <div className="app">
      <header className="titlebar">
        <img src="/portal.png" alt="" className="portal" />
        <h1>
          NETHER<span className="version">{version && `v${version}`}</span>
        </h1>
        <span className={`state-dot ${status.state}`} title={status.state} />
      </header>

      <main className="content">
        <div className="view" data-active={tab === "home"}>
          <Home
            status={status}
            busy={busy}
            onToggle={toggle}
            onOpenSettings={() => setTab("settings")}
          />
        </div>
        <div className="view" data-active={tab === "logs"}>
          <Logs active={tab === "logs"} />
        </div>
        <div className="view" data-active={tab === "settings"}>
          <Settings />
        </div>
      </main>

      <nav className="tabs">
        {(["home", "logs", "settings"] as Tab[]).map((t) => (
          <UnstyledButton
            key={t}
            className={`tab-btn ${t === tab ? "active" : ""}`}
            onClick={() => setTab(t)}
          >
            {ICONS[t]}
            <span>{LABELS[t]}</span>
          </UnstyledButton>
        ))}
      </nav>
    </div>
  );
}
