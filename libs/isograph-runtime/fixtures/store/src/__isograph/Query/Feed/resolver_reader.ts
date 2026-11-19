import type { ReaderArtifact } from '@isograph/runtime';
import type { Query__Feed__output_type } from './output_type';
import type { Query__Feed__param } from './param_type';
import { Feed as resolver } from './../../../Feed';

const readerArtifact = (): ReaderArtifact<
  Query__Feed__param,
  Query__Feed__output_type,
  'Query'
> => ({
  readerAst: [
    {
      kind: 'Linked',
      alias: 'feed',
      selectableName: 'feed',
      arguments: [
        [
          'limit',
          {
            kind: 'Variable',
            name: 'limit',
          },
        ],
      ],
      selections: [
        {
          kind: 'Refinement',
          alias: 'asBlogItem',
          condition: {
            kind: 'SelectionEquals',
            entityName: 'BlogItem',
          },
          selections: [
            {
              kind: 'Scalar',
              alias: 'id',
              selectableName: 'id',
              arguments: null,
            },
            {
              kind: 'Scalar',
              alias: 'title',
              selectableName: 'title',
              arguments: null,
            },
            {
              kind: 'Linked',
              alias: 'author',
              selectableName: 'author',
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
          ],
        },
        {
          kind: 'Refinement',
          alias: 'asAdItem',
          condition: {
            kind: 'SelectionEquals',
            entityName: 'AdItem',
          },
          selections: [
            {
              kind: 'Scalar',
              alias: 'advertiser',
              selectableName: 'advertiser',
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
  resolver: resolver,
});
export default readerArtifact;
