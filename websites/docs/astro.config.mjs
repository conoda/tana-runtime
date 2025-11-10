// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'tana',
			social: [
			{
				icon: 'github', label: 'GitHub', href: 'https://github.com/conoda/tana',
			}],
			sidebar: [
				{
					label: 'Getting Started',
					autogenerate: { directory: 'guides' },
				},
				{
					label: 'CLI Reference',
					autogenerate: { directory: 'tana-cli' },
				},
				{
					label: 'Edge Server',
					autogenerate: { directory: 'tana-edge' },
				},
				{
					label: 'API Reference',
					autogenerate: { directory: 'tana-api' },
				},
				{
					label: 'Contributing',
					autogenerate: { directory: 'contributing' },
				},
			],
		}),
	],
});
