import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { DefuserView } from "../types";
import { ExpertHeader } from "./ExpertHeader";
import { ModuleStepper } from "./ModuleStepper";
import { Manual } from "./Manual";

// The expert's screen. Deliberately does NOT show the bomb itself (wires,
// colors, alien phrases, button labels, serial/batteries/CAR/FRK, sabotage
// events) - that's all on the defuser's tablet page (tablet.html) instead.
// The expert only ever sees: elapsed time/strikes, which module is being
// played, and the rulebook for it. Getting the bomb solved requires the
// defuser describing what they see out loud - showing it here too would
// let one person just solve it solo off this screen.
export function Expert() {
  const [view, setView] = useState<DefuserView | null>(null);

  useEffect(() => {
    let disposed = false;
    const unlistenPromise = listen<DefuserView>("state:update", (e) => setView(e.payload));

    // The backend starts emitting as soon as it connects to the ESP32,
    // which can race ahead of this listener registering - and if nothing
    // changes afterward (e.g. still idle), no further event arrives to
    // "catch up" a missed one. Pull whatever the current state already is
    // so we're never stuck waiting on an event that already came and went.
    (async () => {
      try {
        const current = await invoke<DefuserView | null>("get_game_state");
        if (!disposed && current) {
          setView((existing) => existing ?? current);
        }
      } catch (error) {
        console.error("Failed to fetch initial game state:", error);
      }
    })();

    return () => {
      disposed = true;
      unlistenPromise.then((f) => f());
    };
  }, []);

  if (!view) {
    return (
      <div className="defuser expert">
        <div className="casing">
          <p className="waiting-for-bomb">Waiting for bomb…</p>
        </div>
      </div>
    );
  }

  if (view.phase === "Idle") {
    return (
      <div className="defuser expert">
        <div className="casing">
          <ExpertHeader view={view} />
          <div className="center-message">
            <p>Waiting for the defuser to press Button 1 on the bomb.</p>
          </div>
        </div>
      </div>
    );
  }

  if (view.phase === "Defused" || view.phase === "Exploded") {
    return (
      <div className="defuser expert">
        <div className="casing">
          <ExpertHeader view={view} />
          <div className={`center-message result ${view.phase === "Defused" ? "defused" : "exploded"}`}>
            <div className="result-title">{view.phase === "Defused" ? "DEFUSED" : "BOOM"}</div>
          </div>
        </div>
      </div>
    );
  }

  const activeModule = view.active_module_index !== null ? view.modules[view.active_module_index] : null;

  return (
    <div className="defuser expert">
      <div className="casing">
        <ExpertHeader view={view} />
        <ModuleStepper modules={view.modules} activeIndex={view.active_module_index} />
        <Manual
          activeKind={activeModule?.kind ?? null}
          holdThresholdMs={view.hold_threshold_ms}
          simonMaxStages={view.simon_max_stages}
        />
      </div>
    </div>
  );
}
