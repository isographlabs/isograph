import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__ThrowingPet__output_type } from './output_type';
import type { Query__ThrowingPet__param } from './param_type';
import { ThrowingPet as resolver } from './../../../ThrowingPet';
import Pet__ThrowingName__resolver_reader from './../../Pet/ThrowingName/resolver_reader';

const readerArtifact = (): ReaderArtifact<
  Query__ThrowingPet__param,
  Query__ThrowingPet__output_type,
  'Query'
> => ({
  readerAst: [
    {
      kind: 'Linked',
      alias: 'pet',
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
      selections: [
        {
          kind: 'Resolver',
          alias: 'ThrowingName',
          arguments: null,
          readerArtifact: Pet__ThrowingName__resolver_reader,
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
