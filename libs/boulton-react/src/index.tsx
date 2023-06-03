import {
  DataId,
  DataTypeValue,
  Link,
  ROOT_ID,
  getOrCreateCacheForUrl,
  onNextChange,
  store,
} from "./cache";
import { useLazyDisposableState } from "@boulton/react-disposable-state";
import { type PromiseWrapper, useReadPromise } from "./PromiseWrapper";
import React from "react";

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

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField;
export type ReaderAst = ReaderAstNode[];

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

export type ReaderResolverVariant = "Eager" | "Component";
export type ReaderResolverField = {
  kind: "Resolver";
  alias: string;
  resolver: BoultonResolver<any, any, any>;
  variant: ReaderResolverVariant | null;
};

export type FragmentReference<
  TReadFromStore extends Object,
  TResolverResult extends Object,
  TUnwrappedResolverResult extends Object
> = {
  kind: "FragmentReference";
  readerAst: ReaderAst;
  root: DataId;
  resolver: (props: {
    data: TReadFromStore;
    [index: string]: any;
  }) => TResolverResult;
};

export function bDeclare(queryText: TemplateStringsArray) {
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
      resolver: artifact.resolver ?? ((x) => x),
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
  console.log("result of calling read", { response, reference });
  if (response.kind === "MissingData") {
    throw onNextChange();
  } else {
    return reference.resolver(response.data) as any;
  }
}

export function readButDoNotEvaluate<
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
  console.log("result of read but do not evaluate", { response, reference });
  if (response.kind === "MissingData") {
    throw onNextChange();
  } else {
    return response.data as any;
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
      case "Scalar": {
        const value = existingRecord[field.response_name];
        if (value == null) {
          return { kind: "MissingData" };
        }
        target[field.alias ?? field.response_name] = value;
        break;
      }
      case "Linked": {
        const value = existingRecord[field.response_name];
        if (Array.isArray(value)) {
          const results = [];
          for (const item of value) {
            const link = assertLink(item);
            if (link == null) {
              return { kind: "MissingData" };
            }
            const result = readData(field.selections, link?.__link);
            if (result.kind === "MissingData") {
              return { kind: "MissingData" };
            }
            results.push(result.data);
          }
          target[field.alias ?? field.response_name] = results;
          break;
        }
        const link = assertLink(value);
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
      }
      case "Resolver": {
        if (field.variant === "Eager") {
          const data = readData(field.resolver.readerAst, root);
          if (data.kind === "MissingData") {
            return { kind: "MissingData" };
          } else {
            target[field.alias] = field.resolver.resolver(data.data);
          }
        } else if (field.variant === "Component") {
          // const data = readData(field.resolver.readerAst, root);
          const resolver_function = field.resolver.resolver;
          target[field.alias] = (additionalRuntimeProps) => {
            // TODO also incorporate field.type
            const RefReaderForName = getRefReaderForName(field.alias);
            return (
              <RefReaderForName
                reference={{
                  kind: "FragmentReference",
                  readerAst: field.resolver.readerAst,
                  root,
                  resolver: resolver_function,
                }}
                additionalRuntimeProps={additionalRuntimeProps}
              />
            );
          };
        } else {
          const fragmentReference = {
            kind: "FragmentReference",
            readerAst: field.resolver.readerAst,
            root,
            // This is a footgun, I should really figure out a better way to handle this.
            // If you misspell a resolver export (it should be the field name), then this
            // will fall back to x => x, when the app developer intended something else.
            resolver: field.resolver.resolver ?? ((x) => x),
          };
          target[field.alias] = fragmentReference;
        }
        break;
      }
    }
  }
  return { kind: "Success", data: target as any };
}

function assertLink(link: DataTypeValue): Link | undefined {
  if (Array.isArray(link)) {
    throw new Error("Unexpected array");
  }
  if (typeof link === "object") {
    return link;
  }
  if (link == null) {
    return undefined;
  }
  throw new Error("Invalid link");
}

const refReaders: { [index: string]: any } = {};
function getRefReaderForName(name: string) {
  if (refReaders[name] == null) {
    function Component({
      reference,
      additionalRuntimeProps,
    }: {
      reference: FragmentReference<any, any, any>;
      additionalRuntimeProps: any;
    }) {
      console.log("Rendering RefReader:", name);
      const data = readButDoNotEvaluate(reference);
      return reference.resolver({ data, ...additionalRuntimeProps });
    }
    Component.displayName = `RefReader<${name}>`;
    refReaders[name] = Component;
  }
  return refReaders[name];
}

export type BoultonComponentProps<TDataType, TOtherProps = Object> = {
  data: TDataType;
} & TOtherProps;
