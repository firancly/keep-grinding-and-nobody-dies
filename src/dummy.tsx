import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Expert } from "./expert/Expert";
import "./defuser.css";

// Entry point for the desktop app window - the expert's screen. The
// defuser's screen is tablet.html, served separately by the Rust relay.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Expert />
  </StrictMode>
);
