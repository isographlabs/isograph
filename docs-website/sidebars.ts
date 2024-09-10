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
    'introduction',
    'quickstart',
    'workflow',
    'isograph-config',
    'loadable-fields',
    'pagination',
    'refetching',
    'expose-field-directives',
    'isograph-rules',
    'faq',
    {
      type: 'category',
      label: 'How Isograph works',
      items: [
        'how-isograph-works/compiler',
        'how-isograph-works/runtime',
        'how-isograph-works/generated-artifacts',
        'how-isograph-works/babel-plugin',
      ],
    },
    'development-workflow',
    'backlog',
  ],
};

export default sidebars;
