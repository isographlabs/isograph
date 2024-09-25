import { type Repository__RepositoryLink__output_type } from '../../Repository/RepositoryLink/output_type';

import { type Variables } from '@isograph/react';

export type User__RepositoryList__param = {
  data: {
    /**
A list of repositories that the user owns.
    */
    repositories: {
      /**
A list of edges.
      */
      edges: (({
        /**
The item at the end of the edge.
        */
        node: ({
                    /**
The Node ID of the Repository object
          */
id: string,
          RepositoryLink: Repository__RepositoryLink__output_type,
                    /**
The name of the repository.
          */
name: string,
                    /**
The repository's name with owner.
          */
nameWithOwner: string,
                    /**
The description of the repository.
          */
description: (string | null),
                    /**
Returns how many forks there are of this repository in the whole network.
          */
forkCount: number,
          /**
A list of pull requests that have been opened in the repository.
          */
          pullRequests: {
                        /**
Identifies the total count of items in the connection.
            */
totalCount: number,
          },
                    /**
Returns a count of how many stargazers there are on this object
          */
stargazerCount: number,
          /**
A list of users watching the repository.
          */
          watchers: {
                        /**
Identifies the total count of items in the connection.
            */
totalCount: number,
          },
        } | null),
      } | null))[],
    },
  },
  parameters: Variables,
};
