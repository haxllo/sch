import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment: 'jsdom',
    setupFiles: ['./vitest.setup.ts'],
    pool: 'forks',
    fileParallelism: false,
    poolOptions: {
      forks: {
        singleFork: true,
      },
    },
    testTimeout: 15_000,
    hookTimeout: 15_000,
  },
})
