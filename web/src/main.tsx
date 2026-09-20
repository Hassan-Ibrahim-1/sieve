import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@react-sigma/core/lib/style.css";
import "./theme/tokens.css";
import { App } from "./app/App";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
