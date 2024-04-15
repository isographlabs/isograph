import {Repository__RepositoryLink__outputType} from '../../Repository/RepositoryLink/output_type';

export type User__RepositoryList__param = {
  repositories: {
    edges: (({
      node: ({
        id: string,
        RepositoryLink: Repository__RepositoryLink__outputType,
        name: string,
        nameWithOwner: string,
        description: (string | null),
        forkCount: number,
        pullRequests: {
          totalCount: number,
        },
        stargazerCount: number,
        watchers: {
          totalCount: number,
        },
      } | null),
    } | null))[],
  },
};
