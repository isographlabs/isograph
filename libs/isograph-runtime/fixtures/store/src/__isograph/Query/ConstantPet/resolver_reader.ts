import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__ConstantPet__output_type } from './output_type';
import type { Query__ConstantPet__param } from './param_type';
import { ConstantPet as resolver } from './../../../ConstantPet';
import Pet__Constant__resolver_reader from './../../Pet/Constant/resolver_reader';

const readerArtifact = (): ReaderArtifact<
  Query__ConstantPet__param,
  Query__ConstantPet__output_type,
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
          alias: 'Constant',
          arguments: null,
          readerArtifact: Pet__Constant__resolver_reader,
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
