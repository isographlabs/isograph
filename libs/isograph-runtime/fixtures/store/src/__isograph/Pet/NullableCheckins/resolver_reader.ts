import type { ReaderArtifact } from '@isograph/runtime';
import type { Pet__NullableCheckins__output_type } from './output_type';
import type { Pet__NullableCheckins__param } from './param_type';
import { NullableCheckins as resolver } from './../../../NullableCheckins';

const readerArtifact = (): ReaderArtifact<
  Pet__NullableCheckins__param,
  Pet__NullableCheckins__output_type,
  'Pet'
> => ({
  readerAst: [
    {
      kind: 'Linked',
      alias: 'checkins',
      selectableName: 'checkins',
      arguments: [
        [
          'limit',
          {
            kind: 'Variable',
            name: 'n',
          },
        ],
      ],
      selections: [
        {
          kind: 'Scalar',
          alias: 'id',
          selectableName: 'id',
          arguments: null,
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
