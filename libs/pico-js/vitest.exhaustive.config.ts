import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { include: ['test-exhaustive/exhaustive.ts'], testTimeout: 86_400_000 },
});
