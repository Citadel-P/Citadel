import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react-swc';
import tailwindcss from '@tailwindcss/vite';
import svgr from 'vite-plugin-svgr';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  worker: {
    format: 'es',
  },
  plugins: [
    react(),
    tailwindcss(),
    svgr({
      include: '**/*.svg?react',
      svgrOptions: {
        exportType: 'default',
      },
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
      // Monaco 0.55 broke the monaco-worker-manager/worker protocol.
      // Redirect to our patched version that is compatible with the new protocol.
      'monaco-worker-manager/worker': path.resolve(import.meta.dirname, './src/lib/monaco/worker-manager-patch.ts'),
      // Monaco embeds an old DOMPurify copy; npm overrides cannot replace this
      // relative import. Use a separate instance of our patched dependency.
      './dompurify/dompurify.js': path.resolve(import.meta.dirname, './src/lib/monaco/sanitizer.ts'),
    },
  },
});
