import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__PetLists__output_type } from './output_type';
import type { Query__PetLists__param } from './param_type';
import { PetLists as resolver } from './../../../PetLists';

const readerArtifact = (): ReaderArtifact<
  Query__PetLists__param,
  Query__PetLists__output_type,
  'Query'
> => ({
  readerAst: [
    {
      kind: 'Linked',
      alias: 'pets',
      selectableName: 'pets',
      arguments: null,
      selections: [
        {
          kind: 'Scalar',
          alias: 'id',
          selectableName: 'id',
          arguments: null,
        },
        {
          kind: 'Scalar',
          alias: 'name',
          selectableName: 'name',
          arguments: null,
        },
      ],
    },
    {
      kind: 'Linked',
      alias: 'maybePets',
      selectableName: 'maybePets',
      arguments: null,
      selections: [
        {
          kind: 'Scalar',
          alias: 'id',
          selectableName: 'id',
          arguments: null,
        },
        {
          kind: 'Scalar',
          alias: 'name',
          selectableName: 'name',
          arguments: null,
        },
      ],
    },
    {
      kind: 'Linked',
      alias: 'petGrid',
      selectableName: 'petGrid',
      arguments: null,
      selections: [
        {
          kind: 'Scalar',
          alias: 'id',
          selectableName: 'id',
          arguments: null,
        },
        {
          kind: 'Scalar',
          alias: 'name',
          selectableName: 'name',
          arguments: null,
        },
      ],
    },
    {
      kind: 'Linked',
      alias: 'statGrid',
      selectableName: 'statGrid',
      arguments: null,
      selections: [
        {
          kind: 'Scalar',
          alias: 'weight',
          selectableName: 'weight',
          arguments: null,
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
