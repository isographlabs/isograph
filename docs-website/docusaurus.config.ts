import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const lightTheme = prismThemes.github;
lightTheme.plain.backgroundColor = 'rgba(0, 0, 0, 0.02)';

const config: Config = {
  title: 'Isograph',
  tagline: 'Select your components like you select your data — with GraphQL!',
  favicon: 'img/isograph_logo.ico',

  url: 'https://isograph.dev/',
  baseUrl: '/',
  trailingSlash: true,

  organizationName: 'isographlabs',
  projectName: 'isograph',
  deploymentBranch: 'gh-pages',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'throw',

  staticDirectories: ['static'],

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
        },
        blog: {
          showReadingTime: false,
        },
        theme: {
          customCss: './src/css/custom.css',
        },
        gtag: {
          trackingID: 'G-NK6B9SYH0R',
          anonymizeIP: true,
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/isograph_logo.png',

    metadata: [
      {
        name: 'keywords',
        content:
          'Isograph, GraphQL, React, data, JavaScript, framework, compiler',
      },
      {
        name: 'twitter:card',
        content: 'https://isograph.dev/img/isograph_logo.png',
      },
    ],

    navbar: {
      title: 'Isograph',
      logo: {
        alt: 'Isograph Logo',
        src: 'img/isograph_logo.png',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'documentationSidebar',
          position: 'left',
          label: 'Documentation',
        },
        { to: '/blog', label: 'Blog', position: 'left' },
        {
          href: 'https://discord.gg/rjDDwvZR',
          label: 'Discord',
          position: 'right',
        },
        {
          href: 'https://github.com/isographlabs/isograph',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Tutorial',
              to: '/docs/introduction',
            },
            {
              label: 'Quickstart',
              to: '/docs/quickstart',
            },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'Discord',
              href: 'https://discord.gg/rjDDwvZR',
            },
            {
              label: 'Twitter',
              href: 'https://twitter.com/isographlabs',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'Blog',
              to: '/blog',
            },
            {
              label: 'GitHub',
              href: 'https://github.com/isographlabs/isograph',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Isograph Labs.`,
    },
    prism: {
      theme: lightTheme,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
