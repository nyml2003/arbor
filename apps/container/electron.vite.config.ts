import { defineConfig } from "electron-vite";
import solid from "vite-plugin-solid";
import { resolve } from "path";

const repoRoot = resolve(__dirname, "../..");

export default defineConfig({
  main: {
    build: {
      outDir: "dist/main",
    },
  },
  preload: {
    build: {
      outDir: "dist/preload",
    },
  },
  renderer: {
    plugins: [solid()],
    build: {
      outDir: "dist/renderer",
      rollupOptions: {
        input: resolve(__dirname, "src/renderer/index.html"),
      },
    },
    server: {
      fs: {
        allow: [repoRoot],
      },
    },
  },
});
