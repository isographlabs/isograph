import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__PetSummaries__output_type } from './output_type';
import type { Query__PetSummaries__param } from './param_type';
import { PetSummaries as resolver } from './../../../PetSummaries';
import Pet__PetName__resolver_reader from './../../Pet/PetName/resolver_reader';

const readerArtifact = (): ReaderArtifact<
  Query__PetSummaries__param,
  Query__PetSummaries__output_type,
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
          kind: 'Resolver',
          alias: 'PetName',
          arguments: null,
          readerArtifact: Pet__PetName__resolver_reader,
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
