import type { NormalizationAst } from '@isograph/runtime';

const normalizationAst: NormalizationAst<'Query'> = {
  discriminatorResponseKey: '__typename',
  root: {
    entityName: 'Query',
    id: '__ROOT',
  },
  selections: [
    {
      kind: 'Selection',
      responseKey: 'feed____limit___v_limit',
      selectableName: 'feed',
      arguments: [
        [
          'limit',
          {
            kind: 'Variable',
            name: 'limit',
          },
        ],
      ],
      target: {
        entity: {
          kind: 'Discriminator',
        },
        identity: {
          kind: 'Omit',
        },
        selections: [
          {
            kind: 'Conditional',
            condition: {
              kind: 'SelectionEquals',
              entityName: 'AdItem',
            },
            identity: {
              kind: 'Omit',
            },
            selections: [
              {
                kind: 'Selection',
                responseKey: null,
                selectableName: 'advertiser',
                arguments: null,
                target: null,
              },
            ],
          },
          {
            kind: 'Conditional',
            condition: {
              kind: 'SelectionEquals',
              entityName: 'BlogItem',
            },
            identity: {
              kind: 'Select',
              selectableName: 'id',
            },
            selections: [
              {
                kind: 'Selection',
                responseKey: null,
                selectableName: 'id',
                arguments: null,
                target: null,
              },
              {
                kind: 'Selection',
                responseKey: null,
                selectableName: 'author',
                arguments: null,
                target: {
                  entity: {
                    kind: 'Known',
                    entityName: 'Pet',
                  },
                  identity: {
                    kind: 'Select',
                    selectableName: 'id',
                  },
                  selections: [
                    {
                      kind: 'Selection',
                      responseKey: null,
                      selectableName: 'id',
                      arguments: null,
                      target: null,
                    },
                    {
                      kind: 'Selection',
                      responseKey: null,
                      selectableName: 'name',
                      arguments: null,
                      target: null,
                    },
                  ],
                },
              },
              {
                kind: 'Selection',
                responseKey: null,
                selectableName: 'title',
                arguments: null,
                target: null,
              },
            ],
          },
        ],
      },
    },
  ],
};
export default normalizationAst;
