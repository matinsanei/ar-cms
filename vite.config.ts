import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  css: {
    postcss: {
      plugins: [],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      path: path.resolve(__dirname, './src/path-shim.ts'),
    },
  },
  define: {
    'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV || 'development'),
    'process.env.STRAPI_ADMIN_BACKEND_URL': JSON.stringify(''),
    'process.env.STRAPI_ADMIN_AUTH_COOKIE_NAME': JSON.stringify('jwtToken'),
    'process.env.STRAPI_ADMIN_AUTH_COOKIE_PATH': JSON.stringify('/'),
    'process.env.STRAPI_ADMIN_AUTH_COOKIE_DOMAIN': JSON.stringify(''),
    'process.env.STRAPI_TELEMETRY_DISABLED': JSON.stringify('true'),
    'process.platform': JSON.stringify('browser'),
    'process.version': JSON.stringify('v20.0.0'),
    global: 'window',
  },
  server: {
    port: 1337,
    proxy: {
      '/admin': {
        target: 'http://127.0.0.1:8088',
        bypass: (req) => {
          if (req.headers.accept?.includes('text/html')) {
            return '/index.html';
          }
        },
      },
      '/content-type-builder': {
        target: 'http://127.0.0.1:8088',
        bypass: (req) => {
          if (req.headers.accept?.includes('text/html')) {
            return '/index.html';
          }
        },
      },
      '/content-manager': {
        target: 'http://127.0.0.1:8088',
        bypass: (req) => {
          if (req.headers.accept?.includes('text/html')) {
            return '/index.html';
          }
        },
      },
      '/upload': 'http://127.0.0.1:8088',
      '/i18n': 'http://127.0.0.1:8088',
      '/api': 'http://127.0.0.1:8088',
    },
  },
});
