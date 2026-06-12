import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

const configDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [solid()],
  publicDir: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    cssTarget: "chrome120",
    minify: "esbuild",
    rollupOptions: {
      input: {
        overlay: resolve(configDir, "overlay.html"),
        settings: resolve(configDir, "settings.html"),
        toast: resolve(configDir, "toast.html"),
      },
    },
  },
});
