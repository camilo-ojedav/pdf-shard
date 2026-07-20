import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// La UI corre dentro de la WebView de Tauri (WebView2 / WebKit / Android WebView).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: ["es2022", "chrome110", "safari15"],
  },
});
