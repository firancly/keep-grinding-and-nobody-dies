import { useEffect, useState, type ReactNode } from "react";
import type { ModuleState } from "../types";

// Content here is a direct transcription of the exact rules implemented in
// src-tauri/src/engine/{memory,simon,button_module,wires}.rs - keep these in
// sync if those rules ever change. The few numbers that come from
// game_config.toml (hold_threshold_ms, simon_stages) are passed in as props
// instead of hardcoded, so they can't drift out of sync with the config.

type Tab = ModuleState["kind"]; // "Memory" | "Simon" | "Hold" | "Wires"
const TABS: Tab[] = ["Memory", "Simon", "Hold", "Wires"];
const TAB_TITLES: Record<Tab, string> = {
  Memory: "Memory",
  Simon: "Simon Says",
  Hold: "The Button",
  Wires: "Alien Wires",
};

// The expert never sees the bomb itself - only this manual. `activeKind`
// (the module currently being played) auto-selects the matching page each
// time it changes, but the expert can still click another tab to look
// ahead or double check something.
export function Manual({
  activeKind,
  holdThresholdMs,
  simonMaxStages,
}: {
  activeKind: Tab | null;
  holdThresholdMs: number;
  simonMaxStages: number;
}) {
  const [tab, setTab] = useState<Tab>(activeKind ?? "Memory");

  useEffect(() => {
    if (activeKind) setTab(activeKind);
  }, [activeKind]);

  return (
    <div className="manual-panel">
      <nav className="manual-tabs">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            className={`manual-tab ${tab === t ? "is-active" : ""} ${activeKind === t ? "is-live" : ""}`}
            onClick={() => setTab(t)}
          >
            {TAB_TITLES[t]}
            {activeKind === t ? " •" : ""}
          </button>
        ))}
      </nav>

      <div className="manual-body">
        {tab === "Memory" && <MemoryManual />}
        {tab === "Simon" && <SimonManual maxStages={simonMaxStages} />}
        {tab === "Hold" && <HoldManual thresholdMs={holdThresholdMs} />}
        {tab === "Wires" && <WiresManual />}
      </div>
    </div>
  );
}

function Example({ children }: { children: ReactNode }) {
  return (
    <div className="manual-example">
      <div className="manual-example-label">EXAMPLE</div>
      {children}
    </div>
  );
}

function MemoryManual() {
  return (
    <div className="manual-section">
      <p className="manual-intro">
        Positions are the 4 physical buttons, left to right (Position 1–4). The number shown
        and the labels on each position <strong>reshuffle every stage</strong> — always press
        based on the current stage's rule, not what worked last time.
      </p>

      <table className="manual-table">
        <thead>
          <tr><th>Stage</th><th>Display shows</th><th>Press</th></tr>
        </thead>
        <tbody>
          <tr><td rowSpan={3}>1</td><td>1 or 2</td><td>Position 2</td></tr>
          <tr><td>3</td><td>Position 3</td></tr>
          <tr><td>4</td><td>Position 4</td></tr>

          <tr><td rowSpan={3}>2</td><td>1</td><td>Wherever label "4" is now</td></tr>
          <tr><td>2 or 4</td><td>Same position you pressed in Stage 1</td></tr>
          <tr><td>3</td><td>Position 1</td></tr>

          <tr><td rowSpan={4}>3</td><td>1</td><td>Wherever the label you pressed in Stage 2 is now</td></tr>
          <tr><td>2</td><td>Wherever the label you pressed in Stage 1 is now</td></tr>
          <tr><td>3</td><td>Position 3</td></tr>
          <tr><td>4</td><td>Wherever label "4" is now</td></tr>

          <tr><td rowSpan={3}>4</td><td>1</td><td>Same position as Stage 1</td></tr>
          <tr><td>2</td><td>Position 1</td></tr>
          <tr><td>3 or 4</td><td>Same position as Stage 2</td></tr>

          <tr><td rowSpan={4}>5</td><td>1</td><td>Wherever the label from Stage 1 is now</td></tr>
          <tr><td>2</td><td>Wherever the label from Stage 2 is now</td></tr>
          <tr><td>3</td><td>Wherever the label from Stage 4 is now</td></tr>
          <tr><td>4</td><td>Wherever the label from Stage 3 is now</td></tr>
        </tbody>
      </table>

      <Example>
        Stage 1 shows <strong>2</strong> → press <strong>Position 2</strong> (say the label there
        was "3" — remember: "pressed label 3, at Position 2"). Stage 2 shows <strong>4</strong> →
        rule says same position as Stage 1 → press <strong>Position 2</strong> again, even though
        the labels have reshuffled.
      </Example>
    </div>
  );
}

function SimonManual({ maxStages }: { maxStages: number }) {
  return (
    <div className="manual-section">
      <ol className="manual-steps">
        <li>Ask the defuser: does the bomb's serial number contain a vowel (A, E, I, O, U)?</li>
        <li>Check the current strike count (0, 1, or 2+).</li>
        <li>When a color flashes, look it up below and call out the matching button.</li>
      </ol>

      <table className="manual-table">
        <thead><tr><th>Serial</th><th>Strikes</th><th>RED</th><th>BLUE</th><th>GREEN</th><th>YELLOW</th></tr></thead>
        <tbody>
          <tr><td rowSpan={3}>Has a vowel</td><td>0</td><td>Button 2</td><td>Button 1</td><td>Button 4</td><td>Button 3</td></tr>
          <tr><td>1</td><td>Button 4</td><td>Button 3</td><td>Button 2</td><td>Button 1</td></tr>
          <tr><td>2+</td><td>Button 3</td><td>Button 1</td><td>Button 4</td><td>Button 2</td></tr>
          <tr><td rowSpan={3}>No vowel</td><td>0</td><td>Button 2</td><td>Button 4</td><td>Button 3</td><td>Button 1</td></tr>
          <tr><td>1</td><td>Button 1</td><td>Button 2</td><td>Button 4</td><td>Button 3</td></tr>
          <tr><td>2+</td><td>Button 4</td><td>Button 3</td><td>Button 2</td><td>Button 1</td></tr>
        </tbody>
      </table>

      <p className="manual-note">
        The sequence replays from the start and grows by one flash each round the defuser gets
        right, up to {maxStages} flashes total.
      </p>

      <Example>
        Serial "KT4X9B" has no vowel. 0 strikes. The bomb flashes <strong>GREEN</strong>. Table
        says GREEN → Button 3. Tell the defuser: <strong>"press Button 3."</strong>
      </Example>
    </div>
  );
}

function HoldManual({ thresholdMs }: { thresholdMs: number }) {
  const thresholdSeconds = (thresholdMs / 1000).toFixed(1).replace(/\.0$/, "");
  return (
    <div className="manual-section">
      <p className="manual-intro">Look at the button's color and label, then apply the first matching rule:</p>
      <ol className="manual-steps">
        <li>Color is <strong>BLUE</strong> and text says <strong>ABORT</strong> → HOLD.</li>
        <li>More than 1 battery and text says <strong>DETONATE</strong> → TAP.</li>
        <li>Color is <strong>WHITE</strong> and the "CAR" indicator is lit → HOLD.</li>
        <li>More than 2 batteries and the "FRK" indicator is lit → TAP.</li>
        <li>Color is <strong>YELLOW</strong> → HOLD.</li>
        <li>Color is <strong>RED</strong> and text says <strong>HOLD</strong> → TAP.</li>
        <li>None of the above → HOLD.</li>
      </ol>

      <p className="manual-note">
        <strong>TAP</strong> = press and release quickly (under ~{thresholdSeconds}s). <strong>HOLD</strong> =
        keep holding until a colored strip lights up on the countdown timer, then release when
        the timer displays:
      </p>
      <table className="manual-table">
        <thead><tr><th>Strip color</th><th>Release when timer shows</th></tr></thead>
        <tbody>
          <tr><td>BLUE</td><td>a 4 anywhere</td></tr>
          <tr><td>YELLOW</td><td>a 5 anywhere</td></tr>
          <tr><td>WHITE or RED</td><td>a 1 anywhere</td></tr>
        </tbody>
      </table>

      <Example>
        Color YELLOW, 1 battery, CAR unlit. Rules 1–4 don't match. Rule 5 matches (YELLOW) →
        HOLD. The strip lights up BLUE → release exactly when the timer reads e.g. <strong>1:24</strong>{" "}
        or <strong>0:47</strong> (contains a 4).
      </Example>
    </div>
  );
}

function WiresManual() {
  return (
    <div className="manual-section">
      <p className="manual-intro">
        Ask the defuser to read the alien phrase aloud and name the CLASS shown (ALPHA, BETA, or
        OMEGA). Every phrase has 3 words: an action word (always means "cut"), a noun word
        (always means "wire"), and an index word that names a wire position — look it up below.
      </p>

      <table className="manual-table">
        <thead><tr><th>Position</th><th>Index words</th></tr></thead>
        <tbody>
          <tr><td>1</td><td>KEL, PLO, ESH</td></tr>
          <tr><td>2</td><td>DRA, RIM, GRA</td></tr>
          <tr><td>3</td><td>VON, TAK, NOL</td></tr>
          <tr><td>4</td><td>SEK, WU, FEX</td></tr>
        </tbody>
      </table>

      <p className="manual-note">That gives you the <strong>clue position</strong>. Then apply the class:</p>
      <ul className="manual-list">
        <li><strong>ALPHA</strong> → cut the wire at the clue position exactly.</li>
        <li><strong>BETA</strong> → cut the wire one position AFTER the clue position (4 wraps to 1).</li>
        <li><strong>OMEGA</strong> → cut the OPPOSITE wire (1↔3, 2↔4).</li>
      </ul>

      <p className="manual-note">
        The module only arms after every wire has been reconnected — if a wire looks cut, tell
        the defuser to reconnect it first.
      </p>

      <Example>
        Phrase "VRAK DRA ZORP", Class BETA. "DRA" = position 2. BETA → one after → position 3.
        Tell the defuser: <strong>"cut wire 3."</strong>
      </Example>
    </div>
  );
}
