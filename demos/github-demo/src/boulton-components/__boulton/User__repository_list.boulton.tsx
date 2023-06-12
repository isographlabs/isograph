import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { repository_list as resolver } from '../user_repository_list.tsx';
import Repository__repository_link, { ReadOutType as Repository__repository_link__outputType } from './Repository__repository_link.boulton';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "repositories",
    alias: null,
    arguments: {
      "last": "first",
    },
    selections: [
      {
        kind: "Linked",
        response_name: "edges",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Linked",
            response_name: "node",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                response_name: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Resolver",
                alias: "repository_link",
                arguments: null,
                resolver: Repository__repository_link,
                variant: "Component",
              },
              {
                kind: "Scalar",
                response_name: "name",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                response_name: "nameWithOwner",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                response_name: "description",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                response_name: "forkCount",
                alias: null,
                arguments: null,
              },
              {
                kind: "Linked",
                response_name: "pullRequests",
                alias: null,
                arguments: {
                  "first": "first",
                },
                selections: [
                  {
                    kind: "Scalar",
                    response_name: "totalCount",
                    alias: null,
                    arguments: null,
                  },
                ],
              },
              {
                kind: "Scalar",
                response_name: "stargazerCount",
                alias: null,
                arguments: null,
              },
              {
                kind: "Linked",
                response_name: "watchers",
                alias: null,
                arguments: {
                  "first": "first",
                },
                selections: [
                  {
                    kind: "Scalar",
                    response_name: "totalCount",
                    alias: null,
                    arguments: null,
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  repositories: {
    edges: (({
      node: ({
        id: string,
        repository_link: Repository__repository_link__outputType,
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
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
