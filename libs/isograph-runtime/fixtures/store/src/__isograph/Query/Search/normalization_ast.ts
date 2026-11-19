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
      responseKey: 'search____filter___o_minAge__l_2_nameContains__v_q_c',
      selectableName: 'search',
      arguments: [
        [
          'filter',
          {
            kind: 'Object',
            value: [
              [
                'minAge',
                {
                  kind: 'Literal',
                  value: 2,
                },
              ],
              [
                'nameContains',
                {
                  kind: 'Variable',
                  name: 'q',
                },
              ],
            ],
          },
        ],
      ],
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
  ],
};
export default normalizationAst;
