import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from "vite-plugin-singlefile";

export default defineConfig(({ mode }) => ({
  plugins: [react(),
    mode !== "development" && viteSingleFile(),
  ],
  server: {
    proxy: {
      "/api": "http://localhost:8000",
    },
  },
}));
