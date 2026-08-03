// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: 'GPUi Shell',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/andre-brandao/gpui-shell' }],
      head: [
        {
          tag: 'script',
          attrs: {
            src: 'https://a.developing.company/api/script.js',
            'data-site-id': 'dd3d22977696',
            defer: true,
          },
        },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Installation', slug: 'guides/installation' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Configuration', slug: 'reference/configuration' },
            { label: 'Bar', slug: 'reference/bar' },
            { label: 'Launcher', slug: 'reference/launcher' },
            { label: 'OSD', slug: 'reference/osd' },
            { label: 'Control Center', slug: 'reference/control-center' },
            { label: 'Theme', slug: 'reference/theme' },
          ],
        },
      ],
    }),
  ],
});
