import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

export default defineWorkersConfig({
  test: {
    poolOptions: {
      workers: {
        wrangler: {
          configPath: "./wrangler.toml",
        },
        // Isolated storage ensures each test gets fresh KV state
        // This means NO rate limiting between tests!
        isolatedStorage: true,
        // Enable parallel test execution within worker pool
        // Each test file can run in parallel with isolated storage
        singleWorker: false,
      },
    },
    // Test file patterns
    include: ["test/**/*.test.ts"],
    // Increase timeout for worker tests
    testTimeout: 30000,
    // Run test files in parallel
    fileParallelism: true,
  },
});
