import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Porta fissa: Tauri carica l'interfaccia da qui durante lo sviluppo.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2022", outDir: "dist" },
});
