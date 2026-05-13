import { defineConfig } from "vite";

export default defineConfig({
  build: {
    assetsDir: "assets",
    rollupOptions: {
      input: "index.html",
    },
  },
  publicDir: "src/assets",
});
