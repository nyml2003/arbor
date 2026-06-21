import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import { resolve } from "path";

export default defineConfig({
  plugins: [solid()],
  root: resolve(__dirname, "src/web"),
  build: {
    outDir: resolve(__dirname, "dist/web"),
    emptyOutDir: true,
    rollupOptions: {
      input: resolve(__dirname, "src/web/index.html"),
    },
  },
  server: {
    fs: {
      allow: [resolve(__dirname, "../..")],
    },
  },
});
