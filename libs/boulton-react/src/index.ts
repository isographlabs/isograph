import {
  DataId,
  Link,
  ROOT_ID,
  getOrCreateCacheForUrl,
  onNextChange,
  store,
} from "./cache";
import { useLazyDisposableState } from "@boulton/react-disposable-state";
import { type PromiseWrapper, useReadPromise } from "./PromiseWrapper";

// This type should be treated as an opaque type.
export type BoultonFetchableResolver<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
> = {
  kind: "FetchableResolver";
  queryText: string;
  normalizationAst: any;
  readerAst: ReaderAst;
  resolver: (data: TReadFromStore) => TResolverResult;
};

export type BoultonNonFetchableResolver<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
> = {
  kind: "NonFetchableResolver";
  readerAst: ReaderAst;
  resolver: (data: TReadFromStore) => TResolverResult;
};

export type BoultonResolver<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
> =
  | BoultonFetchableResolver<
      TReadFromStore,
      TResolverResult,
      TUnwrappedResolverResult
    >
  | BoultonNonFetchableResolver<
      TReadFromStore,
      TResolverResult,
      TUnwrappedResolverResult
    >;

export type ReaderAst = [
  ReaderScalarField | ReaderLinkedField | ReaderResolverField
];
export type ReaderScalarField = {
  kind: "Scalar";
  response_name: string;
  alias: string | null;
};
export type ReaderLinkedField = {
  kind: "Linked";
  response_name: string;
  alias: string | null;
  selections: ReaderAst;
};
export type ReaderResolverField = {
  kind: "Resolver";
  alias: string;
  resolver: BoultonResolver<any, any, any>;
};

// TODO
// - Resolvers should be eager, lazy and maybe "component", where eager
//   means that you can skip a call to read(...) and just get the data,
//   lazy means that you have to call read(...) to get the data, and
//   component returns something you interpolate which may suspend.

export type FragmentReference<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
> = {
  kind: "FragmentReference";
  readerAst: ReaderAst;
  root: DataId;
  resolver?: (networkResponse: TReadFromStore) => TResolverResult;
};

export function bDeclare(queryText: string) {
  return (x: any) => x;
}

export function useLazyReference<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
>(
  artifact: BoultonFetchableResolver<
    TReadFromStore,
    TResolverResult,
    TUnwrappedResolverResult
  >
): {
  queryReference: FragmentReference<
    TReadFromStore,
    TResolverResult,
    TUnwrappedResolverResult
  >;
} {
  // Typechecking fails here... TODO investigate
  const cache = getOrCreateCacheForUrl<{}>(artifact.queryText, {});
  const data =
    useLazyDisposableState<PromiseWrapper<TUnwrappedResolverResult>>(
      cache
    ).state;

  return {
    queryReference: {
      kind: "FragmentReference",
      readerAst: artifact.readerAst,
      root: ROOT_ID,
    },
  };
}

export function read<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
>(
  reference: FragmentReference<
    TReadFromStore,
    TResolverResult,
    TUnwrappedResolverResult
  >
): TUnwrappedResolverResult {
  const response = readData<TReadFromStore>(
    reference.readerAst,
    reference.root
  );
  if (response.kind === "MissingData") {
    throw onNextChange();
  } else {
    if (reference.resolver != null) {
      return reference.resolver(response.data) as any;
    } else {
      return response.data as any;
    }
  }
}

type ReadDataResult<TReadFromStore> =
  | {
      kind: "Success";
      data: TReadFromStore;
    }
  | {
      kind: "MissingData";
    };

function readData<TReadFromStore>(
  ast: ReaderAst,
  root: DataId
): ReadDataResult<TReadFromStore> {
  let existingRecord = store[root];
  if (existingRecord == null) {
    return { kind: "MissingData" };
  }

  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case "Scalar":
        const value = existingRecord[field.response_name];
        if (value == null) {
          return { kind: "MissingData" };
        }
        target[field.alias ?? field.response_name] = value;
        break;
      case "Linked":
        const link = assertLink(existingRecord[field.response_name]);
        if (link == null) {
          return { kind: "MissingData" };
        }
        const targetId = link.__link;
        const data = readData(field.selections, targetId);
        if (data.kind === "MissingData") {
          return { kind: "MissingData" };
        }
        target[field.alias ?? field.response_name] = data.data;
        break;
      case "Resolver":
        target[field.alias] = {
          kind: "FragmentReference",
          readerAst: field.resolver.readerAst,
          root,
          resolver: field.resolver.resolver,
        };
        break;
    }
  }
  return { kind: "Success", data: target as any };
}

function assertLink(link: Link | string | undefined): Link | undefined {
  if (typeof link === "object") {
    return link;
  }
  if (link == null) {
    return undefined;
  }
  throw new Error("Invalid link");
}
