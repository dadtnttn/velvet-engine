// Vitest settings: the tests sit next to the code, in src/, and their names end in .test.ts.

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
  },
});
