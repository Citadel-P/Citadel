import { mergeConfig } from 'vite';
import { defineConfig } from 'vitest/config';
import viteConfig from './vite.config';

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      globals: true,
      setupFiles: ['./src/test/setup.ts'],
      clearMocks: true,
      mockReset: true,
      restoreMocks: true,
      coverage: {
        provider: 'v8',
        reporter: ['text', 'json-summary', 'html'],
        include: ['src/**/*.{ts,tsx}'],
        exclude: [
          'src/api/generated/**',
          'src/components/ui/**',
          'src/**/*.d.ts',
          'src/**/index.ts',
          'src/**/icons.tsx',
          'src/main.tsx',
          'src/test/**',
        ],
      },
    },
  }),
);
