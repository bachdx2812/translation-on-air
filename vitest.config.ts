import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Frontend unit tests run in jsdom. Test files live next to the code they cover
// (src/**/*.test.ts[x]).
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
