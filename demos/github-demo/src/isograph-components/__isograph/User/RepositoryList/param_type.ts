import { type Repository__RepositoryLink__output_type } from '../../Repository/RepositoryLink/output_type';

import { type Variables } from '@isograph/react';

export type User__RepositoryList__param = {
  readonly data: {
    /**
A list of repositories that the user owns.
    */
    readonly repositories: {
      /**
A list of edges.
      */
      readonly edges: (ReadonlyArray<({
        /**
The item at the end of the edge.
        */
        readonly node: ({
                    /**
The Node ID of the Repository object
          */
readonly id: string,
          readonly RepositoryLink: Repository__RepositoryLink__output_type,
                    /**
The name of the repository.
          */
readonly name: string,
                    /**
The repository's name with owner.
          */
readonly nameWithOwner: string,
                    /**
The description of the repository.
          */
readonly description: (string | null),
                    /**
Returns how many forks there are of this repository in the whole network.
          */
readonly forkCount: number,
          /**
A list of pull requests that have been opened in the repository.
          */
          readonly pullRequests: {
                        /**
Identifies the total count of items in the connection.
            */
readonly totalCount: number,
          },
                    /**
Returns a count of how many stargazers there are on this object
          */
readonly stargazerCount: number,
          /**
A list of users watching the repository.
          */
          readonly watchers: {
                        /**
Identifies the total count of items in the connection.
            */
readonly totalCount: number,
          },
        } | null),
      } | null)> | null),
    },
  },
  readonly parameters: Variables,
};
