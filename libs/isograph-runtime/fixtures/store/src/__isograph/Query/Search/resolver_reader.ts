import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__Search__output_type } from './output_type';
import type { Query__Search__param } from './param_type';
import { Search as resolver } from './../../../Search';

const readerArtifact = (): ReaderArtifact<
  Query__Search__param,
  Query__Search__output_type,
  'Query'
> => ({
  readerAst: [
    {
      kind: 'Linked',
      alias: 'search',
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
  ],
  resolver: resolver,
});
export default readerArtifact;
