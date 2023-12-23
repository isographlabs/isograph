import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { repository_page as resolver } from '../../../isograph-components/repository.tsx';
import Query__header, { ReadOutType as Query__header__outputType } from '../header/reader.isograph';
import Query__repository_detail, { ReadOutType as Query__repository_detail__outputType } from '../repository_detail/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "header",
    arguments: null,
    readerArtifact: Query__header,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "repository_detail",
    arguments: null,
    readerArtifact: Query__repository_detail,
    usedRefetchQueries: [],
  },
];

export type ResolverParameterType = { data:
{
  header: Query__header__outputType,
  repository_detail: Query__repository_detail__outputType,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.repository_page" },
};

export default artifact;
