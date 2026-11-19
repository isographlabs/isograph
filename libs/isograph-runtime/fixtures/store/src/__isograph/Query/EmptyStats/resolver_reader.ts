import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__EmptyStats__output_type } from './output_type';
import type { Query__EmptyStats__param } from './param_type';
import { EmptyStats as resolver } from './../../../EmptyStats';

const readerArtifact = (): ReaderArtifact<
  Query__EmptyStats__param,
  Query__EmptyStats__output_type,
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
          kind: 'Linked',
          alias: 'stats',
          selectableName: 'stats',
          arguments: null,
          selections: [],
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
