import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import { ThemeProvider } from "./hooks/useTheme.tsx";

// Initialize WASM before importing App (which imports trident-core)
// This ensures WASM is ready before any code tries to use it
async function initApp() {
  // With --target bundler, wasm-pack exports a default async init function
  // Import trident-core and initialize WASM
  const tridentCore = await import("trident-core");
  
  // If there's a default export (init function), call it
  // Otherwise, the module is already initialized (dev mode or web target)
  const initFn = tridentCore.default;
  if (initFn && typeof initFn === "function") {
    await (initFn as () => Promise<void>)();
  }
  
  // Now import App after WASM is initialized
  const { default: App } = await import("./App.tsx");
  
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <ThemeProvider>
        <App />
      </ThemeProvider>
    </StrictMode>
  );
}

initApp();