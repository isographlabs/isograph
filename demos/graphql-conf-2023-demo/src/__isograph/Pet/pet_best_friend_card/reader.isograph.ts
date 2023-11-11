import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { pet_best_friend_card as resolver } from '../../../components/pet_best_friend_card.tsx';
import Pet__best_friend_selector, { ReadOutType as Pet__best_friend_selector__outputType } from '../best_friend_selector/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Resolver",
    alias: "best_friend_selector",
    arguments: null,
    readerArtifact: Pet__best_friend_selector,
    usedRefetchQueries: [0, ],
  },
  {
    kind: "Linked",
    fieldName: "best_friend_relationship",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "picture_together",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "best_friend",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "picture",
            alias: null,
            arguments: null,
          },
        ],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  id: string,
  best_friend_selector: Pet__best_friend_selector__outputType,
  best_friend_relationship: ({
    picture_together: (string | null),
    best_friend: {
      id: string,
      name: string,
      picture: string,
    },
  } | null),
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.pet_best_friend_card" },
};

export default artifact;
