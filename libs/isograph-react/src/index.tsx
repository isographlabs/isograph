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

export {
  setNetwork,
  makeNetworkRequest,
  subscribe,
  DataId,
  Link,
  StoreRecord,
} from "./cache";

// This type should be treated as an opaque type.
export type IsographEntrypoint<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "Entrypoint";
  queryText: string;
  normalizationAst: NormalizationAst;
  readerArtifact: ReaderArtifact<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  >;
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};

export type ReaderArtifact<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
> = {
  kind: "ReaderArtifact";
  readerAst: ReaderAst<TReadFromStore>;
  resolver: (data: TResolverProps) => TResolverResult;
  variant: ReaderResolverVariant;
};

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

export type ReaderResolverVariant =
  | { kind: "Eager" }
  // componentName is the component's cacheKey for getRefReaderByName
  // and is the type + field concatenated
  | { kind: "Component"; componentName: string };

export type ReaderResolverField = {
  kind: "Resolver";
  alias: string;
  readerArtifact: ReaderArtifact<any, any, any>;
  arguments: Arguments | null;
  usedRefetchQueries: number[];
};

export type ReaderRefetchField = {
  kind: "RefetchField";
  alias: string;
  // TODO this bad modeling. A refetch field cannot have variant: "Component" (I think)
  readerArtifact: ReaderArtifact<any, any, any>;
  refetchQuery: number;
};

export type ReaderMutationField = {
  kind: "MutationField";
  alias: string;
  // TODO this bad modeling. A mutation field cannot have variant: "Component" (I think)
  readerArtifact: ReaderArtifact<any, any, any>;
  refetchQuery: number;
  allowedVariables: string[];
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
  arguments: Arguments | null;
  selections: NormalizationAst;
};

// This is more like an entrypoint, but one specifically for a refetch query/mutation
export type RefetchQueryArtifact = {
  kind: "RefetchQuery";
  queryText: string;
  normalizationAst: NormalizationAst;
};

// TODO rename
export type RefetchQueryArtifactWrapper = {
  artifact: RefetchQueryArtifact;
  allowedVariables: string[];
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
  readerArtifact: ReaderArtifact<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  >;
  root: DataId;
  variables: { [index: string]: string } | null;
  // TODO: We should instead have ReaderAst<TResolverProps>
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};

// export function iso<T extends IsographEntrypoint<any, any, any>>(
//   _text: TemplateStringsArray
// ): T {
//   throw new Error("Babel plugin not enabled")
// }

export function iso<TResolverParameter, TResolverReturn = TResolverParameter>(
  _queryText: TemplateStringsArray
): (
  x: ((param: TResolverParameter) => TResolverReturn) ) => (param: TResolverParameter) => TResolverReturn {
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

function assertIsEntrypoint<TReadFromStore extends Object,
  TResolverProps,
  TResolverResult>(value: IsographEntrypoint<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  > | typeof iso): asserts value is IsographEntrypoint<TReadFromStore, TResolverProps, TResolverResult> {
  if (typeof value === 'function') throw new Error("Not a string")
}

export function useLazyReference<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  > | typeof iso,
  variables: object
): {
  queryReference: FragmentReference<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  >;
} {

  // Typechecking fails here... TODO investigate
  assertIsEntrypoint(entrypoint);
  const cache = getOrCreateCacheForArtifact(entrypoint, variables);

  // TODO add comment explaining why we never use this value
  // @ts-ignore
  const data =
    // @ts-ignore
    useLazyDisposableState<PromiseWrapper<TResolverResult>>(cache).state;

  return {
    queryReference: {
      kind: "FragmentReference",
      readerArtifact: entrypoint.readerArtifact,
      root: ROOT_ID,
      variables,
      nestedRefetchQueries: entrypoint.nestedRefetchQueries,
    },
  };
}

export function read<
  TReadFromStore extends Object,
  TResolverProps,
  TResolverResult
>(
  fragmentReference: FragmentReference<
    TReadFromStore,
    TResolverProps,
    TResolverResult
  >
): TResolverResult {
  const variant = fragmentReference.readerArtifact.variant;
  if (variant.kind === "Eager") {
    const data = readData(
      fragmentReference.readerArtifact.readerAst,
      fragmentReference.root,
      fragmentReference.variables ?? {},
      fragmentReference.nestedRefetchQueries
    );
    if (data.kind === "MissingData") {
      throw onNextChange();
    } else {
      return fragmentReference.readerArtifact.resolver(data.data);
    }
  } else if (variant.kind === "Component") {
    return (additionalRuntimeProps: any) => {
      // TODO also incorporate the typename
      const RefReaderForName = getRefReaderForName(variant.componentName);
      // TODO do not create a new reference on every render?
      return (
        <RefReaderForName
          reference={{
            kind: "FragmentReference",
            readerArtifact: fragmentReference.readerArtifact,
            root: fragmentReference.root,
            variables: fragmentReference.variables,
            nestedRefetchQueries: fragmentReference.nestedRefetchQueries,
          }}
          additionalRuntimeProps={additionalRuntimeProps}
        />
      );
    };
  }
  // Why can't Typescript realize that this is unreachable??
  throw new Error("This is unreachable");
}

export function readButDoNotEvaluate<TReadFromStore extends Object>(
  reference: FragmentReference<TReadFromStore, unknown, unknown>
): TReadFromStore {
  const response = readData(
    reference.readerArtifact.readerAst,
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
  nestedRefetchQueries: RefetchQueryArtifactWrapper[]
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
          const altLink = missingFieldHandler(
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
          field.readerArtifact.readerAst,
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
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error("refetchQuery is null in RefetchField");
          }
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          target[field.alias] = field.readerArtifact.resolver(
            refetchQueryArtifact,
            {
              ...data.data,
              // TODO continue from here
              // variables need to be filtered for what we need just for the refetch query
              ...filterVariables(variables, allowedVariables),
            }
          );
        }
        break;
      }
      case "MutationField": {
        const data = readData(
          field.readerArtifact.readerAst,
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
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error("refetchQuery is null in MutationField");
          }
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          target[field.alias] = field.readerArtifact.resolver(
            refetchQueryArtifact,
            data.data,
            filterVariables(variables, allowedVariables)
          );
        }
        break;
      }
      case "Resolver": {
        const usedRefetchQueries = field.usedRefetchQueries;
        const resolverRefetchQueries = usedRefetchQueries.map(
          (index) => nestedRefetchQueries[index]
        );

        const variant = field.readerArtifact.variant;
        if (variant.kind === "Eager") {
          const data = readData(
            field.readerArtifact.readerAst,
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
            target[field.alias] = field.readerArtifact.resolver(data.data);
          }
        } else if (variant.kind === "Component") {
          target[field.alias] = (additionalRuntimeProps: any) => {
            // TODO also incorporate the typename
            const RefReaderForName = getRefReaderForName(variant.componentName);
            // TODO do not create a new reference on every render?
            return (
              <RefReaderForName
                reference={{
                  kind: "FragmentReference",
                  readerArtifact: field.readerArtifact,
                  root,
                  variables,
                  nestedRefetchQueries: resolverRefetchQueries,
                }}
                additionalRuntimeProps={additionalRuntimeProps}
              />
            );
          };
        }
        break;
      }
    }
  }
  return { kind: "Success", data: target as any };
}

let customMissingFieldHandler: typeof defaultMissingFieldHandler | null = null;

function missingFieldHandler(
  storeRecord: StoreRecord,
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null
): Link | undefined {
  if (customMissingFieldHandler != null) {
    return customMissingFieldHandler(
      storeRecord,
      root,
      fieldName,
      arguments_,
      variables
    );
  } else {
    return defaultMissingFieldHandler(
      storeRecord,
      root,
      fieldName,
      arguments_,
      variables
    );
  }
}

export function defaultMissingFieldHandler(
  storeRecord: StoreRecord,
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null
): Link | undefined {
  if (fieldName === "node" || fieldName === "user") {
    const variable = arguments_?.["id"];
    const value = variables?.[variable];

    // TODO can we handle explicit nulls here too? Probably, after wrapping in objects
    if (value != null) {
      return { __link: value };
    }
  }
}

export function setMissingFieldHandler(
  handler: typeof defaultMissingFieldHandler
) {
  customMissingFieldHandler = handler;
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

      return reference.readerArtifact.resolver({
        data,
        ...additionalRuntimeProps,
      });
    }
    Component.displayName = `${name} @component`;
    refReaders[name] = Component;
  }
  return refReaders[name];
}

export type IsographComponentProps<TDataType, TOtherProps = Object> = {
  data: TDataType;
} & TOtherProps;

function filterVariables(
  variables: { [index: string]: string },
  allowedVariables: string[]
): { [index: string]: string } {
  const result: { [index: string]: string } = {};
  for (const key of allowedVariables) {
    result[key] = variables[key];
  }
  return result;
}
