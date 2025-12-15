import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */
const sidebars: SidebarsConfig = {
  documentationSidebar: [
    'index',
    'quickstart',
    'workflow',
    'isograph-config',
    'loadable-fields',
    'pagination',
    'mutations',
    'local-updates',
    'external-data-sources',
    'conditional-fetching',
    'abstract-types',
    'data-driven-dependencies',
    'parameters',
    'faq',
    {
      type: 'category',
      label: 'How Isograph works',
      items: [
        'how-isograph-works/compiler',
        'how-isograph-works/runtime',
        'how-isograph-works/generated-artifacts',
        'how-isograph-works/babel-plugin',
        'how-isograph-works/one-pager',
        'how-isograph-works/compiler-one-pager',
      ],
    },
    {
      type: 'category',
      label: 'Design docs',
      items: [
        'design-docs/incremental-compilation',
        'design-docs/isograph-data-model',
      ],
    },
    {
      type: 'category',
      label: 'Miscellaneous',
      items: [
        'isograph-rules',
        'development-workflow',
        'backlog',
        'troubleshooting',
      ],
    },
    {
      type: 'category',
      label: 'Deprecated features',
      items: ['expose-field-directives', 'refetching'],
    },
  ],
};

export default sidebars;
