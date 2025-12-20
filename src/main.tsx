import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import wasmUrl from "trident-core/trident_core_bg.wasm?url";

// Initialize WASM before rendering

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);

if (!import.meta.env.DEV) {
  const init = await import('trident-core').then((module) => module.default);
  (init as any)(wasmUrl).then(() => {
    createRoot(document.getElementById("root")!).render(
      <StrictMode>
        <App />
      </StrictMode>
    );
  });
}