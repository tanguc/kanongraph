import {themes as prismThemes} from 'prism-react-renderer';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'MonPhare',
  tagline: 'Catch version drift, deprecated modules, and risky constraints across all your Terraform repos',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://tanguc.github.io',
  baseUrl: '/monphare/',

  organizationName: 'tanguc',
  projectName: 'monphare',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          editUrl: 'https://github.com/tanguc/monphare/tree/main/website/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      colorMode: {
        defaultMode: 'dark',
        respectPrefersColorScheme: true,
      },
      navbar: {
        title: 'MonPhare',
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docsSidebar',
            position: 'left',
            label: 'Docs',
          },
          {
            href: 'https://github.com/tanguc/monphare',
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
              {label: 'Getting Started', to: '/docs/getting-started/installation'},
              {label: 'Scanning', to: '/docs/scanning/local-directories'},
              {label: 'Configuration', to: '/docs/configuration/overview'},
            ],
          },
          {
            title: 'More',
            items: [
              {label: 'GitHub', href: 'https://github.com/tanguc/monphare'},
              {label: 'Releases', href: 'https://github.com/tanguc/monphare/releases'},
            ],
          },
        ],
        copyright: `Copyright ${new Date().getFullYear()} MonPhare. MIT License.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['bash', 'hcl', 'yaml', 'rust', 'json'],
      },
    }),
};

export default config;
