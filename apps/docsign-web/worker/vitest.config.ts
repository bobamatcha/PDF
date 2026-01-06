import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

export default defineWorkersConfig({
  test: {
    // Run tests sequentially to avoid KV conflicts
    poolOptions: {
      workers: {
        wrangler: {
          configPath: "./wrangler.toml",
        },
        // Isolated storage ensures each test gets fresh KV state
        // This means NO rate limiting between tests!
        isolatedStorage: true,
        // Single threaded for predictable test execution
        singleWorker: true,
      },
    },
    // Test file patterns
    include: ["test/**/*.test.ts"],
    // Increase timeout for worker tests
    testTimeout: 30000,
  },
});
