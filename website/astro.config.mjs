import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import tailwindcss from '@tailwindcss/vite';
import cloudflare from '@astrojs/cloudflare';

export default defineConfig({
  integrations: [svelte()],
  output: 'server',  // SSR mode, but specific pages can be prerendered with `export const prerender = true`
  adapter: cloudflare(),

  build: {
    inlineStylesheets: 'auto'
  },

  vite: {
    plugins: [tailwindcss()]
  }
});