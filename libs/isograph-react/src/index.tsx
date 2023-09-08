import {
  DataId,
  StoreRecord,
  DataTypeValue,
  Link,
  ROOT_ID,
  getOrCreateCacheForArtifact,
  onNextChange,
  store,
  getParentRecordKey,
} from "./cache";
import { useLazyDisposableState } from "@isograph/react-disposable-state";
import { type PromiseWrapper } from "./PromiseWrapper";
import React from "react";
import nodeQuery from "./node_query";

export { setNetwork, makeNetworkRequest, subscribe } from "./cache";

// This type should be treated as an opaque type.
export type IsographFetchableResolver<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "FetchableResolver";
  queryText: string;
  normalizationAst: NormalizationAst;
  readerAst: ReaderAst<TReadFromStore>;
  resolver: (data: TResolverProps) => TResolverResult;
  // TODO ReaderAst<TResolverProps> should contain the convert function
  convert: (
    resolver: (data: TResolverProps) => TResolverResult,
    data: TReadFromStore
  ) => TResolverResult; // TODO this should be a different return type
  nestedRefetchQueries: RefetchQueryArtifact[];
};

export type IsographNonFetchableResolver<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "NonFetchableResolver";
  readerAst: ReaderAst<TReadFromStore>;
  resolver: (data: TResolverProps) => TResolverResult;
};

export type IsographResolver<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> =
  | IsographFetchableResolver<TReadFromStore, TResolverProps, TResolverResult>
  | IsographNonFetchableResolver<
      TReadFromStore,
      TResolverProps,
      TResolverResult
    >;

export type ReaderAstNode =
  | ReaderScalarField
  | ReaderLinkedField
  | ReaderResolverField
  | ReaderRefetchField
  | ReaderMutationField;

// @ts-ignore
export type ReaderAst<TReadFromStore> = ReaderAstNode[];

export type ReaderScalarField = {
  kind: "Scalar";
  fieldName: string;
  alias: string | null;
  arguments: Arguments | null;
};
export type ReaderLinkedField = {
  kind: "Linked";
  fieldName: string;
  alias: string | null;
  selections: ReaderAst<unknown>;
  arguments: Arguments | null;
};

export type ReaderResolverVariant = "Eager" | "Component";
export type ReaderResolverField = {
  kind: "Resolver";
  alias: string;
  resolver: IsographResolver<any, any, any>;
  variant: ReaderResolverVariant | null;
  arguments: Arguments | null;
  usedRefetchQueries: number[];
};

export type ReaderRefetchField = {
  kind: "RefetchField";
  alias: string;
  resolver: IsographResolver<any, any, any>;
  refetchQuery: number;
};

export type ReaderMutationField = {
  kind: "MutationField";
  alias: string;
  resolver: IsographResolver<any, any, any>;
  refetchQuery: number;
};

export type NormalizationAstNode =
  | NormalizationScalarField
  | NormalizationLinkedField;
// @ts-ignore
export type NormalizationAst = NormalizationAstNode[];

export type NormalizationScalarField = {
  kind: "Scalar";
  fieldName: string;
  arguments: Arguments | null;
};

export type NormalizationLinkedField = {
  kind: "Linked";
  fieldName: string;
  // TODO consider getting rid of, and always deriving this from arguments
  alias: string | null;
  arguments: Arguments | null;
  selections: NormalizationAst;
};

export type RefetchQueryArtifact = {
  kind: "RefetchQuery";
  queryText: string;
  normalizationAst: NormalizationAst;
};

export type Arguments = Argument[];
export type Argument = {
  argumentName: string;
  variableName: string;
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
  variables: { [index: string]: string } | null;
  // TODO: We should instead have ReaderAst<TResolverProps>
  convert: (
    resolver: (data: TResolverProps) => TResolverResult,
    data: TReadFromStore
  ) => TResolverResult;
  nestedRefetchQueries: RefetchQueryArtifact[];
};

export function iso<TResolverParameter, TResolverReturn = TResolverParameter>(
  _queryText: TemplateStringsArray
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
  artifact: IsographFetchableResolver<
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
  const cache = getOrCreateCacheForArtifact(artifact, variables);

  // TODO add comment explaining why we never use this value
  // @ts-ignore
  const data =
    // @ts-ignore
    useLazyDisposableState<PromiseWrapper<TResolverResult>>(cache).state;
  return {
    queryReference: {
      kind: "FragmentReference",
      readerAst: artifact.readerAst,
      root: ROOT_ID,
      convert: artifact.convert,
      resolver: artifact.resolver,
      variables,
      nestedRefetchQueries: artifact.nestedRefetchQueries,
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
    reference.variables ?? {},
    reference.nestedRefetchQueries
  );
  console.log("done reading", { response });
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
    reference.variables ?? {},
    reference.nestedRefetchQueries
  );
  console.log("done reading but not evaluating", { response });
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
      reason: string;
      nestedReason?: ReadDataResult<unknown>;
    };

function readData<TReadFromStore>(
  ast: ReaderAst<TReadFromStore>,
  root: DataId,
  variables: { [index: string]: string },
  nestedRefetchQueries: RefetchQueryArtifact[]
): ReadDataResult<TReadFromStore> {
  let storeRecord = store[root];
  if (storeRecord === undefined) {
    return { kind: "MissingData", reason: "No record for root " + root };
  }

  if (storeRecord === null) {
    return { kind: "Success", data: null as any };
  }

  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case "Scalar": {
        const storeRecordName = getParentRecordKey(field, variables);
        const value = storeRecord[storeRecordName];
        // TODO consider making scalars into discriminated unions. This probably has
        // to happen for when we handle errors.
        if (value === undefined) {
          return {
            kind: "MissingData",
            reason: "No value for " + storeRecordName + " on root " + root,
          };
        }
        target[field.alias ?? field.fieldName] = value;
        break;
      }
      case "Linked": {
        const storeRecordName = getParentRecordKey(field, variables);
        const value = storeRecord[storeRecordName];
        if (Array.isArray(value)) {
          const results = [];
          for (const item of value) {
            const link = assertLink(item);
            if (link === undefined) {
              return {
                kind: "MissingData",
                reason:
                  "No link for " +
                  storeRecordName +
                  " on root " +
                  root +
                  ". Link is " +
                  JSON.stringify(item),
              };
            } else if (link === null) {
              results.push(null);
              continue;
            }
            const result = readData(
              field.selections,
              link.__link,
              variables,
              nestedRefetchQueries
            );
            if (result.kind === "MissingData") {
              return {
                kind: "MissingData",
                reason:
                  "Missing data for " +
                  storeRecordName +
                  " on root " +
                  root +
                  ". Link is " +
                  JSON.stringify(item),
                nestedReason: result,
              };
            }
            results.push(result.data);
          }
          target[field.alias ?? field.fieldName] = results;
          break;
        }
        let link = assertLink(value);
        if (link === undefined) {
          // TODO make this configurable, and also generated and derived from the schema
          const altLink = HACK_missingFieldHandler(
            storeRecord,
            root,
            field.fieldName,
            field.arguments,
            variables
          );
          if (altLink === undefined) {
            return {
              kind: "MissingData",
              reason:
                "No link for " +
                storeRecordName +
                " on root " +
                root +
                ". Link is " +
                JSON.stringify(value),
            };
          } else {
            link = altLink;
          }
        } else if (link === null) {
          target[field.alias ?? field.fieldName] = null;
          break;
        }
        const targetId = link.__link;
        const data = readData(
          field.selections,
          targetId,
          variables,
          nestedRefetchQueries
        );
        if (data.kind === "MissingData") {
          return {
            kind: "MissingData",
            reason: "Missing data for " + storeRecordName + " on root " + root,
            nestedReason: data,
          };
        }
        target[field.alias ?? field.fieldName] = data.data;
        break;
      }
      case "RefetchField": {
        const data = readData(
          field.resolver.readerAst,
          root,
          variables,
          // Refetch fields just read the id, and don't need refetch query artifacts
          []
        );
        console.log("refetch field data", data, field);
        if (data.kind === "MissingData") {
          return {
            kind: "MissingData",
            reason: "Missing data for " + field.alias + " on root " + root,
            nestedReason: data,
          };
        } else {
          // TODO do we also need to call convert?
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error("refetchQuery is null in RefetchField");
          }
          const refetchQueryArtifact = nestedRefetchQueries[refetchQueryIndex];

          target[field.alias] = field.resolver.resolver(refetchQueryArtifact, {
            ...data.data,
            // TODO continue from here
            // variables need to be filtered for what we need just for the refetch query
            ...variables,
          });
        }
        break;
      }
      case "MutationField": {
        const data = readData(
          field.resolver.readerAst,
          root,
          variables,
          // Refetch fields just read the id, and don't need refetch query artifacts
          []
        );
        console.log("refetch field data", data, field);
        if (data.kind === "MissingData") {
          return {
            kind: "MissingData",
            reason: "Missing data for " + field.alias + " on root " + root,
            nestedReason: data,
          };
        } else {
          // TODO do we also need to call convert?
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error("refetchQuery is null in MutationField");
          }
          console.log("mutation field", data.data, variables);
          const refetchQueryArtifact = nestedRefetchQueries[refetchQueryIndex];

          target[field.alias] = field.resolver.resolver(refetchQueryArtifact, {
            ...data.data,
            // TODO continue from here
            // variables need to be filtered for what we need just for the mutation query
            ...variables,
          });
        }
        break;
      }
      case "Resolver": {
        const usedRefetchQueries = field.usedRefetchQueries;
        const resolverRefetchQueries = usedRefetchQueries.map(
          (index) => nestedRefetchQueries[index]
        );

        if (field.variant === "Eager") {
          const data = readData(
            field.resolver.readerAst,
            root,
            variables,
            resolverRefetchQueries
          );
          if (data.kind === "MissingData") {
            return {
              kind: "MissingData",
              reason: "Missing data for " + field.alias + " on root " + root,
              nestedReason: data,
            };
          } else {
            // // Does this go here??

            // TODO do we also need to call convert?
            target[field.alias] = field.resolver.resolver(data.data);
          }
        } else if (field.variant === "Component") {
          // const data = readData(field.resolver.readerAst, root);
          const resolver_function = field.resolver.resolver;
          target[field.alias] = (additionalRuntimeProps: any) => {
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
                    // Change refReader props to not accept this
                    console.log(new Error().stack);
                    throw new Error("where did I convert 1 ");
                  },
                  nestedRefetchQueries: resolverRefetchQueries,
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
            //
            // lint rules will ameliorate this
            resolver: field.resolver.resolver ?? ((x) => x),
            convert: (resolver: any, data: any) => resolver(data),
            nestedRefetchQueries: resolverRefetchQueries,
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
  storeRecord: StoreRecord,
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null
): Link | undefined {
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

    // TODO can we handle explicit nulls here too? Probably, after wrapping in objects
    if (value != null) {
      console.log("found node", value);
      return { __link: value };
    }
  }
}

function assertLink(link: DataTypeValue): Link | undefined | null {
  if (Array.isArray(link)) {
    throw new Error("Unexpected array");
  }
  if (typeof link === "object") {
    return link;
  }
  if (link === undefined) {
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
      const data = readButDoNotEvaluate(reference);
      return reference.resolver({ data, ...additionalRuntimeProps });
    }
    Component.displayName = `${name} @component`;
    refReaders[name] = Component;
  }
  return refReaders[name];
}

export function getRefRendererForName(name: string) {
  // TODO name should be Type__fieldName, right now it's just fieldName.
  // Collisions on the fieldName will cause problems.
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

export type IsographComponentProps<TDataType, TOtherProps = Object> = {
  data: TDataType;
} & TOtherProps;
