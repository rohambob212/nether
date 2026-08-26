import { useEffect, useState } from "react";
import { api, type EngineStatus } from "./api";
import Home from "./views/Home";
import Logs from "./views/Logs";
import Settings from "./views/Settings";

type Tab = "home" | "logs" | "settings";

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
        {tab === "home" && (
          <Home
            status={status}
            busy={busy}
            onToggle={toggle}
            onOpenSettings={() => setTab("settings")}
          />
        )}
        {tab === "logs" && <Logs />}
        {tab === "settings" && <Settings />}
      </main>

      <nav className="tabs">
        {(["home", "logs", "settings"] as Tab[]).map((t) => (
          <button key={t} className={t === tab ? "active" : ""} onClick={() => setTab(t)}>
            {t.toUpperCase()}
          </button>
        ))}
      </nav>
    </div>
  );
}
