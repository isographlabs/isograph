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
      responseKey: null,
      selectableName: 'greeting',
      arguments: null,
      target: null,
    },
    {
      kind: 'Selection',
      responseKey:
        'greeting____loud___l_true____name___s_Ada_Lovelace____times___l_3',
      selectableName: 'greeting',
      arguments: [
        [
          'loud',
          {
            kind: 'Literal',
            value: true,
          },
        ],
        [
          'name',
          {
            kind: 'String',
            value: 'Ada Lovelace',
          },
        ],
        [
          'times',
          {
            kind: 'Literal',
            value: 3,
          },
        ],
      ],
      target: null,
    },
    {
      kind: 'Selection',
      responseKey: 'greeting____name___v_name',
      selectableName: 'greeting',
      arguments: [
        [
          'name',
          {
            kind: 'Variable',
            name: 'name',
          },
        ],
      ],
      target: null,
    },
    {
      kind: 'Selection',
      responseKey: 'greeting____name___l_null',
      selectableName: 'greeting',
      arguments: [
        [
          'name',
          {
            kind: 'Literal',
            value: null,
          },
        ],
      ],
      target: null,
    },
    {
      kind: 'Selection',
      responseKey: null,
      selectableName: 'settings',
      arguments: null,
      target: null,
    },
    {
      kind: 'Selection',
      responseKey: null,
      selectableName: 'tags',
      arguments: null,
      target: null,
    },
  ],
};
export default normalizationAst;
