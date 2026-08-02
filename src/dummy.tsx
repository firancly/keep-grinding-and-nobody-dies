import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Defuser } from "./defuser/Defuser";
import "./defuser.css";

// Entry point for the Defuser ("dummy") window.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Defuser />
  </StrictMode>
);
