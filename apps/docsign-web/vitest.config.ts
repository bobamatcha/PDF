import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/ts/__tests__/**/*.test.ts'],
    environment: 'node',
    globals: true,
  },
});
