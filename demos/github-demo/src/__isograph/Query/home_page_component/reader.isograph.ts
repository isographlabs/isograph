import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { home_page_component as resolver } from '../../../isograph-components/home_page.tsx';
import Query__header, { ReadOutType as Query__header__outputType } from '../header/reader.isograph';
import Query__home_page_list, { ReadOutType as Query__home_page_list__outputType } from '../home_page_list/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

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
    alias: "home_page_list",
    arguments: null,
    readerArtifact: Query__home_page_list,
    usedRefetchQueries: [0, ],
  },
];

export type ResolverParameterType = { data:
{
  header: Query__header__outputType,
  home_page_list: Query__home_page_list__outputType,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.home_page_component" },
};

export default artifact;
