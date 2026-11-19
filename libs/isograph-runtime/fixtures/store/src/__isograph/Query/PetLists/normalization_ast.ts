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
      selectableName: 'maybePets',
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
      selectableName: 'petGrid',
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
      selectableName: 'pets',
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
      selectableName: 'statGrid',
      arguments: null,
      target: {
        entity: {
          kind: 'Known',
          entityName: 'PetStats',
        },
        identity: {
          kind: 'Omit',
        },
        selections: [
          {
            kind: 'Selection',
            responseKey: null,
            selectableName: 'weight',
            arguments: null,
            target: null,
          },
        ],
      },
    },
  ],
};
export default normalizationAst;
