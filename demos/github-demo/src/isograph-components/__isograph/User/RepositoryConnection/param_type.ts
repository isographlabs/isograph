import { type Repository__RepositoryRow__output_type } from '../../Repository/RepositoryRow/output_type';
import type { User__RepositoryConnection__parameters } from './parameters_type';

export type User__RepositoryConnection__param = {
  readonly data: {
    /**
A list of repositories that the user owns.
    */
    readonly repositories: {
      /**
Information to aid in pagination.
      */
      readonly pageInfo: {
        /**
When paginating forwards, are there more items?
        */
        readonly hasNextPage: boolean,
        /**
When paginating forwards, the cursor to continue.
        */
        readonly endCursor: (string | null),
      },
      /**
A list of edges.
      */
      readonly edges: (ReadonlyArray<({
        /**
The item at the end of the edge.
        */
        readonly node: ({
          readonly RepositoryRow: Repository__RepositoryRow__output_type,
          /**
The Node ID of the Repository object
          */
          readonly id: string,
        } | null),
      } | null)> | null),
    },
  },
  readonly parameters: User__RepositoryConnection__parameters,
};
