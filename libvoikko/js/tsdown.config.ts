import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  dts: true,
  clean: true,
  hash: false,
  external: [
    'node:fs',
    'node:fs/promises',
    'node:path',
    /\/wasm\//,
  ],
});
