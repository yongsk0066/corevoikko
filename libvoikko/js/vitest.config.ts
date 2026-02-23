import { defineConfig } from 'vitest/config';
import pkg from './package.json' with { type: 'json' };

export default defineConfig({
  define: {
    __PKG_VERSION__: JSON.stringify(pkg.version),
  },
  test: {
    globals: true,
    globalSetup: ['test/setup-dict.ts'],
  },
});
