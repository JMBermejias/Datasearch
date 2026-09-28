// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
import { defineConfig } from "vite";

// Rutas relativas: necesario para que la aplicacion funcione al abrirse desde
// el sistema de ficheros y no solo desde el servidor de desarrollo.
export default defineConfig({
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "0.0.0.0",
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
    minify: "esbuild",
    rollupOptions: {
      input: {
        principal: "index.html",
        actualizacion: "actualizacion.html",
      },
    },
  },
});
