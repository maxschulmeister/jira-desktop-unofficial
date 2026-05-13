import { defineConfig } from "vite";

export default defineConfig({
  root: "src/",
  build: {
    assetsDir: "assets",
    rollupOptions: {
      input: "src/index.html",
    },
  },
  publicDir: "src/assets",
});
