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
      responseKey: 'pet____id___v_id',
      selectableName: 'pet',
      arguments: [
        [
          'id',
          {
            kind: 'Variable',
            name: 'id',
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
            responseKey: 'checkins____limit___l_null',
            selectableName: 'checkins',
            arguments: [
              [
                'limit',
                {
                  kind: 'Literal',
                  value: null,
                },
              ],
            ],
            target: {
              entity: {
                kind: 'Known',
                entityName: 'Checkin',
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
              ],
            },
          },
        ],
      },
    },
  ],
};
export default normalizationAst;
