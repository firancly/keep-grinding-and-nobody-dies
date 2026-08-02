import { useEffect, useState } from "react";
import type { SimonColor, SimonModule } from "../../types";

const PADS: SimonColor[] = ["red", "blue", "green", "yellow"];

// Plays the flash_sequence back on a loop so the defuser can read it off the
// screen. The physical buttons + ESP handle input; a fresh payload arrives
// (with a bumped input_len) after each press.
export function Simon({ mod }: { mod: SimonModule }) {
  const [lit, setLit] = useState<SimonColor | null>(null);

  useEffect(() => {
    if (mod.solved || mod.flash_sequence.length === 0) return;
    let i = 0;
    let alive = true;
    const step = () => {
      if (!alive) return;
      setLit(mod.flash_sequence[i]);
      setTimeout(() => {
        if (!alive) return;
        setLit(null);
        i = (i + 1) % mod.flash_sequence.length;
      }, 400);
    };
    step();
    const id = setInterval(step, 700);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [mod.flash_sequence, mod.solved]);

  return (
    <div className="simon">
      <div className="simon-pads">
        {PADS.map((c) => (
          <span
            key={c}
            className={`simon-pad ${lit === c ? "is-lit" : ""}`}
            data-color={c}
          />
        ))}
      </div>
      <div className="simon-progress">
        {mod.flash_sequence.map((_, i) => (
          <span
            key={i}
            className={`pip ${i < mod.input_len ? "is-done" : ""}`}
          />
        ))}
      </div>
    </div>
  );
}
