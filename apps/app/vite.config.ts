import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
  server: {
    port: parseInt(process.env.PORT || "5174"),
    host: true,
  },
});
