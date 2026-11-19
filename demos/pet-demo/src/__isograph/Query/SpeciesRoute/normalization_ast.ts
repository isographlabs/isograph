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
      responseKey: 'node____id___s_1',
      selectableName: 'node',
      arguments: [
        [
          'id',
          {
            kind: 'String',
            value: '1',
          },
        ],
      ],
      target: {
        entity: {
          kind: 'Discriminator',
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
    {
      kind: 'Selection',
      responseKey: 'petsBySpecies____species___v_species',
      selectableName: 'petsBySpecies',
      arguments: [
        [
          'species',
          {
            kind: 'Variable',
            name: 'species',
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
            selectableName: 'favorite_phrase',
            arguments: null,
            target: null,
          },
          {
            kind: 'Selection',
            responseKey: null,
            selectableName: 'firstName',
            arguments: null,
            target: null,
          },
          {
            kind: 'Selection',
            responseKey: null,
            selectableName: 'lastName',
            arguments: null,
            target: null,
          },
        ],
      },
    },
    {
      kind: 'Selection',
      responseKey: null,
      selectableName: 'viewer',
      arguments: null,
      target: {
        entity: {
          kind: 'Known',
          entityName: 'Viewer',
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
};
export default normalizationAst;
