import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

const requireEnv = (name: string): string => {
  const value = process.env[name];
  if (!value) throw new Error(`${name} env variable is required`);
  return value;
};

export default defineConfig({
  plugins: [tailwindcss()],
  server: {
    port: parseInt(requireEnv("PORT")),
    host: true,
    allowedHosts: ['app.adi.test'],
  },
});
