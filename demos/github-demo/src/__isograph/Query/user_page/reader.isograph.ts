import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { user_page as resolver } from '../../../isograph-components/user.tsx';
import Query__header, { ReadOutType as Query__header__outputType } from '../header/reader.isograph';
import Query__user_detail, { ReadOutType as Query__user_detail__outputType } from '../user_detail/reader.isograph';

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
    alias: "user_detail",
    arguments: null,
    readerArtifact: Query__user_detail,
    usedRefetchQueries: [],
  },
];

export type ResolverParameterType = { data:
{
  header: Query__header__outputType,
  user_detail: Query__user_detail__outputType,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.user_page" },
};

export default artifact;
