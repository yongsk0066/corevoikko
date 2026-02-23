import { defineConfig } from 'tsdown';
import pkg from './package.json' with { type: 'json' };

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  dts: true,
  clean: true,
  hash: false,
  define: {
    __PKG_VERSION__: JSON.stringify(pkg.version),
  },
  external: [
    'node:fs',
    'node:fs/promises',
    'node:path',
    'node:url',
    /\/wasm\//,
  ],
});
