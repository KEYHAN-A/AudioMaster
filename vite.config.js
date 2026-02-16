import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [
    vue(),
    {
      name: "strip-crossorigin",
      enforce: "post",
      transformIndexHtml(html) {
        return html.replace(/ crossorigin/g, "");
      },
    },
  ],
  base: "./",
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
