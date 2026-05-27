import { defineConfig } from "vite";

export default defineConfig({
  root: "src/",

  publicDir: "assets",

  build: {
    outDir: "../dist",
    emptyOutDir: true,
    assetsDir: "assets",
    rollupOptions: {
      input: "src/index.html",
    },
  },
});
