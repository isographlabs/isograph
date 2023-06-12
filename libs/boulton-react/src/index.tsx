import {
  DataId,
  DataType,
  DataTypeValue,
  Link,
  ROOT_ID,
  getOrCreateCacheForUrl,
  onNextChange,
  store,
} from "./cache";
import { useLazyDisposableState } from "@boulton/react-disposable-state";
import { type PromiseWrapper } from "./PromiseWrapper";
import React from "react";

export { setNetwork } from "./cache";

// This type should be treated as an opaque type.
export type BoultonFetchableResolver<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "FetchableResolver";
  queryText: string;
  normalizationAst: any;
  readerAst: ReaderAst<TReadFromStore>;
  resolver: (data: TResolverProps) => TResolverResult;
  // TODO ReaderAst<TResolverProps> should contain the convert function
  convert: (
    resolver: (data: TResolverProps) => TResolverResult,
    data: TReadFromStore
  ) => TResolverResult; // TODO this should be a different return type
};

export type BoultonNonFetchableResolver<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "NonFetchableResolver";
  readerAst: ReaderAst<TReadFromStore>;
  resolver: (data: TResolverProps) => TResolverResult;
};

export type BoultonResolver<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> =
  | BoultonFetchableResolver<TReadFromStore, TResolverProps, TResolverResult>
  | BoultonNonFetchableResolver<
      TReadFromStore,
      TResolverProps,
      TResolverResult
    >;

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField;
export type ReaderAst<TReadFromStore> = ReaderAstNode[];

export type ReaderScalarField = {
  kind: "Scalar";
  response_name: string;
  alias: string | null;
  arguments: Object | null;
};
export type ReaderLinkedField = {
  kind: "Linked";
  response_name: string;
  alias: string | null;
  selections: ReaderAst<unknown>;
  arguments: Object | null;
};

export type ReaderResolverVariant = "Eager" | "Component";
export type ReaderResolverField = {
  kind: "Resolver";
  alias: string;
  resolver: BoultonResolver<any, any, any>;
  variant: ReaderResolverVariant | null;
  arguments: Object | null;
};

export type FragmentReference<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "FragmentReference";
  readerAst: ReaderAst<TReadFromStore>;
  root: DataId;
  resolver: (props: TResolverProps) => TResolverResult;
  variables: Object | null;
  // TODO: We should instead have ReaderAst<TResolverProps>
  convert: (
    resolver: (data: TResolverProps) => TResolverResult,
    data: TReadFromStore
  ) => TResolverResult;
};

export function bDeclare<
  TResolverParameter,
  TResolverReturn = TResolverParameter
>(
  queryText: TemplateStringsArray
): (
  x: ((param: TResolverParameter) => TResolverReturn) | void
) => (param: TResolverParameter) => TResolverReturn {
  // The name `identity` here is a bit of a double entendre.
  // First, it is the identity function, constrained to operate
  // on a very specific type. Thus, the value of b Declare`...`(
  // someFunction) is someFunction. But furthermore, if one
  // write b Declare`...` and passes no function, the resolver itself
  // is the identity function. At that point, the types
  // TResolverParameter and TResolverReturn must be identical.

  return function identity(
    x: (param: TResolverParameter) => TResolverReturn
  ): (param: TResolverParameter) => TResolverReturn {
    return x;
  };
}

export function useLazyReference<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
>(
  artifact: BoultonFetchableResolver<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  >,
  variables: object
): {
  queryReference: FragmentReference<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  >;
} {
  // Typechecking fails here... TODO investigate
  const cache = getOrCreateCacheForUrl(artifact.queryText, variables);
  const data =
    useLazyDisposableState<PromiseWrapper<TResolverResult>>(cache).state;
  return {
    queryReference: {
      kind: "FragmentReference",
      readerAst: artifact.readerAst,
      root: ROOT_ID,
      convert: artifact.convert,
      resolver: artifact.resolver,
      variables,
    },
  };
}

export function read<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
>(
  reference: FragmentReference<TReadFromStore, TResolverProps, TResolverResult>
): TResolverResult {
  const response = readData(
    reference.readerAst,
    reference.root,
    reference.variables
  );
  console.log("result of calling read", { response, reference });
  if (response.kind === "MissingData") {
    throw onNextChange();
  } else {
    return reference.convert(reference.resolver, response.data);
  }
}

export function readButDoNotEvaluate<TReadFromStore extends Object>(
  reference: FragmentReference<TReadFromStore, unknown, unknown>
): TReadFromStore {
  const response = readData(
    reference.readerAst,
    reference.root,
    reference.variables
  );
  console.log("result of read but do not evaluate", { response, reference });
  if (response.kind === "MissingData") {
    throw onNextChange();
  } else {
    return response.data;
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
  ast: ReaderAst<TReadFromStore>,
  root: DataId,
  variables: Object | null
): ReadDataResult<TReadFromStore> {
  let storeRecord = store[root];
  if (storeRecord == null) {
    return { kind: "MissingData" };
  }

  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case "Scalar": {
        const storeRecordName = getStoreFieldName(
          field.response_name,
          field.arguments,
          variables
        );
        const value = storeRecord[storeRecordName];
        if (value == null) {
          return { kind: "MissingData" };
        }
        target[field.alias ?? field.response_name] = value;
        break;
      }
      case "Linked": {
        const storeRecordName = getStoreFieldName(
          field.response_name,
          field.arguments,
          variables
        );
        const value = storeRecord[storeRecordName];
        if (Array.isArray(value)) {
          const results = [];
          for (const item of value) {
            const link = assertLink(item);
            if (link == null) {
              return { kind: "MissingData" };
            }
            const result = readData(field.selections, link.__link, variables);
            if (result.kind === "MissingData") {
              return { kind: "MissingData" };
            }
            results.push(result.data);
          }
          target[field.alias ?? field.response_name] = results;
          break;
        }
        let link = assertLink(value);
        if (link == null) {
          // TODO make this configurable, and also generated and derived from the schema
          const altLink = HACK_missingFieldHandler(
            storeRecord,
            root,
            field.response_name,
            field.arguments,
            variables
          );
          if (altLink == null) {
            return { kind: "MissingData" };
          } else {
            link = altLink;
          }
        }
        const targetId = link.__link;
        const data = readData(field.selections, targetId, variables);
        if (data.kind === "MissingData") {
          return { kind: "MissingData" };
        }
        target[field.alias ?? field.response_name] = data.data;
        break;
      }
      case "Resolver": {
        if (field.variant === "Eager") {
          const data = readData(field.resolver.readerAst, root, variables);
          if (data.kind === "MissingData") {
            return { kind: "MissingData" };
          } else {
            // TODO do we also need to call convert?
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
                  variables,
                  convert: () => {
                    console.log(new Error().stack);
                    throw new Error("where did I convert");
                  },
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
            convert: () => {
              console.log(new Error().stack);
              throw new Error("where did I convert");
            },
          };
          target[field.alias] = fragmentReference;
        }
        break;
      }
    }
  }
  return { kind: "Success", data: target as any };
}

function HACK_missingFieldHandler(
  storeRecord: DataType,
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null
): Link | null {
  console.log("missing field handler", {
    storeRecord,
    root,
    fieldName,
    arguments_,
    variables,
  });
  if (fieldName === "node" || fieldName === "user") {
    const variable = arguments_?.["id"];
    const value = variables?.[variable];

    if (value != null) {
      console.log("found node", value);
      return { __link: value };
    }
  }
  return null;
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
export function getRefReaderForName(name: string) {
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
    Component.displayName = `${name} @component`;
    refReaders[name] = Component;
  }
  return refReaders[name];
}

export function getRefRendererForName(name: string) {
  if (refReaders[name] == null) {
    function Component({
      resolver,
      data,
      additionalRuntimeProps,
    }: {
      resolver: any;
      additionalRuntimeProps: any;
      data: any;
    }) {
      return resolver({ data, ...additionalRuntimeProps });
    }
    Component.displayName = `${name} @component`;
    refReaders[name] = Component;
  }
  return refReaders[name];
}

export type BoultonComponentProps<TDataType, TOtherProps = Object> = {
  data: TDataType;
} & TOtherProps;

function getStoreFieldName(
  name: string,
  args: { [index: string]: any } | null,
  variables: { [index: string]: any } | null
): string {
  if (args === null) {
    return name;
  }
  if (variables == null) {
    throw new Error("Missing variables when args are present");
  }

  let keys = Object.keys(args ?? {});
  keys.sort();

  if (keys.length === 0) {
    return name;
  } else {
    let out = name;
    for (const key of keys) {
      if (variables[args[key]] == null) {
        throw new Error("Undefined variable " + args[key]);
      }
      out = out + "__" + key + "_" + variables[args[key]];
    }
    return out;
  }
}
