/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Getting Started',
      collapsed: false,
      items: [
        'getting-started/installation',
        'getting-started/quick-start',
      ],
    },
    {
      type: 'category',
      label: 'Scanning',
      collapsed: false,
      items: [
        'scanning/local-directories',
        'scanning/remote-repositories',
        'scanning/org-scale',
        'scanning/filtering-and-excluding',
        'scanning/output-formats',
        'scanning/ci-cd',
      ],
    },
    {
      type: 'category',
      label: 'Commands',
      items: [
        'commands/scan',
        'commands/graph',
        'commands/init',
        'commands/validate',
      ],
    },
    {
      type: 'category',
      label: 'Configuration',
      items: [
        'configuration/overview',
        'configuration/scan-options',
        'configuration/analysis-options',
        'configuration/policies',
        'configuration/deprecations',
        'configuration/cache',
      ],
    },
    {
      type: 'category',
      label: 'Findings',
      items: [
        'findings/overview',
        'findings/missing-version',
        'findings/broad-constraint',
        'findings/wildcard-constraint',
        'findings/no-upper-bound',
        'findings/exact-version',
        'findings/prerelease-version',
      ],
    },
    {
      type: 'category',
      label: 'API',
      items: [
        'api/rust-crate',
      ],
    },
  ],
};

export default sidebars;
