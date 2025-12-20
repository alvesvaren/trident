import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { spawn, type ChildProcess } from "node:child_process";
import path from "node:path";

let cargo: ChildProcess | null = null;

export default defineConfig({
  plugins: [
    {
      name: "cargo-watch",
      configureServer({ httpServer, watcher, ws }) {
        if (cargo) return;

        cargo = spawn("cargo", ["watch", "--", "wasm-pack", "build"], {
          cwd: "./trident-core",
          stdio: "inherit",
        });

        httpServer?.on("close", () => cargo?.kill("SIGINT"));

        watcher.on("change", (file) => {
          if (file.includes("trident-core/pkg")) {
            ws.send({
              type: "full-reload",
            });
          }
        });
      },
    },
    react({
      babel: {
        plugins: [["babel-plugin-react-compiler"]],
      },
    }),
    wasm(),
    topLevelAwait(),
  ],
  resolve: {
    alias: {
      "trident-core": path.resolve(__dirname, "trident-core/pkg"),
    },
  },
  optimizeDeps: {
    exclude: ["trident-core", "@syntect/wasm"],
  }
});
