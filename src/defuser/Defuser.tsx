import { useEffect, useState } from "react";
import type { DefuserView } from "../types";
import { mockDefuser } from "../mock";
import { StatusBar } from "./StatusBar";
import { ModuleCard } from "./ModuleCard";
import { EventOverlay } from "./EventOverlay";

export function Defuser() {
  const [view, setView] = useState<DefuserView>(mockDefuser);

  // --- MOCK DRIVERS (remove once Rust emits state:update) -------------------
  // Tick the timer so the UI feels live during styling/demo.
  useEffect(() => {
    const id = setInterval(() => {
      setView((v) => ({ ...v, timer_ms: Math.max(0, v.timer_ms - 1000) }));
    }, 1000);
    return () => clearInterval(id);
  }, []);
  // -------------------------------------------------------------------------

  // When the backend is ready, replace the block above with:
  //   import { listen } from "@tauri-apps/api/event";
  //   useEffect(() => {
  //     const un = listen<DefuserView>("state:update", (e) => setView(e.payload));
  //     return () => { un.then((f) => f()); };
  //   }, []);

  const event = view.active_event;
  const rootEffect = event ? `fx-${event.kind.toLowerCase()}` : "";

  return (
    <div className={`defuser ${rootEffect}`}>
      <div className="casing">
        <StatusBar view={view} />
        <main className="module-grid">
          {view.modules.map((mod, i) => (
            <ModuleCard key={i} mod={mod} />
          ))}
        </main>
      </div>
      {event && <EventOverlay event={event} />}
    </div>
  );
}
