import {
  getOrCreateCacheForArtifact,
  onNextChange,
  getParentRecordKey,
  subscribe,
} from './cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { type PromiseWrapper } from './PromiseWrapper';
import { getOrCreateCachedComponent } from './componentCache';
import {
  DataId,
  DataTypeValue,
  IsographEnvironment,
  Link,
  ROOT_ID,
  StoreRecord,
} from './IsographEnvironment';
import { useEffect, useState } from 'react';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ReaderArtifact, ReaderAst } from './reader';
import {
  IsographEntrypoint,
  RefetchQueryArtifactWrapper,
  assertIsEntrypoint,
} from './entrypoint';

export {
  retainQuery,
  unretainQuery,
  type RetainedQuery,
  garbageCollectEnvironment,
} from './garbageCollection';
export { type PromiseWrapper } from './PromiseWrapper';
export { makeNetworkRequest, subscribe } from './cache';
export {
  ROOT_ID,
  type DataId,
  type DataTypeValue,
  type IsographEnvironment,
  type IsographNetworkFunction,
  type IsographStore,
  type Link,
  type StoreRecord,
  createIsographEnvironment,
  createIsographStore,
} from './IsographEnvironment';
export {
  IsographEnvironmentProvider,
  useIsographEnvironment,
  type IsographEnvironmentProviderProps,
} from './IsographEnvironmentProvider';
export { useImperativeReference } from './useImperativeReference';
export { EntrypointReader } from './EntrypointReader';
export {
  type ReaderArtifact,
  ReaderAst,
  ReaderAstNode,
  ReaderLinkedField,
  ReaderMutationField,
  ReaderRefetchField,
  ReaderResolverField,
  ReaderResolverVariant,
  ReaderScalarField,
} from './reader';
export {
  NormalizationAst,
  NormalizationAstNode,
  NormalizationLinkedField,
  NormalizationScalarField,
  IsographEntrypoint,
  assertIsEntrypoint,
  RefetchQueryArtifact,
  RefetchQueryArtifactWrapper,
} from './entrypoint';

export type ExtractSecondParam<T extends (arg1: any, arg2: any) => any> =
  T extends (arg1: any, arg2: infer P) => any ? P : never;

export type Arguments = Argument[];
export type Argument = [ArgumentName, ArgumentValue];
export type ArgumentName = string;
export type ArgumentValue =
  | {
      kind: 'Variable';
      name: string;
    }
  | {
      kind: 'Literal';
      value: any;
    };

// TODO type this better
type Variable = any;

export type FragmentReference<
  TReadFromStore extends Object,
  TResolverResult,
> = {
  kind: 'FragmentReference';
  readerArtifact: ReaderArtifact<TReadFromStore, TResolverResult>;
  root: DataId;
  variables: { [index: string]: Variable } | null;
  // TODO: We should instead have ReaderAst<TResolverProps>
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};

export type ExtractReadFromStore<Type> =
  Type extends IsographEntrypoint<infer X, any> ? X : never;
export type ExtractResolverResult<Type> =
  Type extends IsographEntrypoint<any, infer X> ? X : never;
// Note: we cannot write TEntrypoint extends IsographEntrypoint<any, any, any>, or else
// if we do not explicitly pass a type, the read out type will be any.
// We cannot write TEntrypoint extends IsographEntrypoint<never, never, never>, or else
// any actual Entrypoint we pass will not be valid.
export function useLazyReference<TEntrypoint>(
  entrypoint:
    | TEntrypoint
    // Temporarily, we need to allow useLazyReference to take the result of calling
    // iso(`...`). At runtime, we confirm that the passed-in `iso` literal is actually
    // an entrypoint.
    | ((_: any) => any),
  variables: { [key: string]: Variable },
): {
  queryReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >;
} {
  const environment = useIsographEnvironment();
  assertIsEntrypoint<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >(entrypoint);
  const cache = getOrCreateCacheForArtifact<ExtractResolverResult<TEntrypoint>>(
    environment,
    entrypoint,
    variables,
  );

  // TODO add comment explaining why we never use this value
  // @ts-ignore
  const data =
    useLazyDisposableState<PromiseWrapper<ExtractResolverResult<TEntrypoint>>>(
      cache,
    ).state;

  return {
    queryReference: {
      kind: 'FragmentReference',
      readerArtifact: entrypoint.readerArtifact,
      root: ROOT_ID,
      variables,
      nestedRefetchQueries: entrypoint.nestedRefetchQueries,
    },
  };
}

export function useResult<TReadFromStore extends Object, TResolverResult>(
  fragmentReference: FragmentReference<TReadFromStore, TResolverResult>,
): TResolverResult {
  const environment = useIsographEnvironment();

  const [, setState] = useState<object | void>();
  useEffect(() => {
    return subscribe(environment, () => {
      return setState({});
    });
  }, []);

  return read(environment, fragmentReference);
}

export function read<TReadFromStore extends Object, TResolverResult>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, TResolverResult>,
): TResolverResult {
  const variant = fragmentReference.readerArtifact.variant;
  if (variant.kind === 'Eager') {
    const data = readData(
      environment,
      fragmentReference.readerArtifact.readerAst,
      fragmentReference.root,
      fragmentReference.variables ?? {},
      fragmentReference.nestedRefetchQueries,
    );
    if (data.kind === 'MissingData') {
      throw onNextChange(environment);
    } else {
      // @ts-expect-error This not properly typed yet
      return fragmentReference.readerArtifact.resolver(data.data);
    }
  } else if (variant.kind === 'Component') {
    // @ts-ignore
    return getOrCreateCachedComponent(
      environment,
      fragmentReference.root,
      variant.componentName,
      fragmentReference.readerArtifact,
      fragmentReference.variables ?? {},
      fragmentReference.nestedRefetchQueries,
    );
  }
  // Why can't Typescript realize that this is unreachable??
  throw new Error('This is unreachable');
}

export function readButDoNotEvaluate<TReadFromStore extends Object>(
  environment: IsographEnvironment,
  reference: FragmentReference<TReadFromStore, unknown>,
): TReadFromStore {
  const response = readData(
    environment,
    reference.readerArtifact.readerAst,
    reference.root,
    reference.variables ?? {},
    reference.nestedRefetchQueries,
  );
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('done reading', { response });
  }
  if (response.kind === 'MissingData') {
    throw onNextChange(environment);
  } else {
    return response.data;
  }
}

type ReadDataResult<TReadFromStore> =
  | {
      kind: 'Success';
      data: TReadFromStore;
    }
  | {
      kind: 'MissingData';
      reason: string;
      nestedReason?: ReadDataResult<unknown>;
    };

function readData<TReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: DataId,
  variables: { [index: string]: string },
  nestedRefetchQueries: RefetchQueryArtifactWrapper[],
): ReadDataResult<TReadFromStore> {
  let storeRecord = environment.store[root];
  if (storeRecord === undefined) {
    return { kind: 'MissingData', reason: 'No record for root ' + root };
  }

  if (storeRecord === null) {
    return { kind: 'Success', data: null as any };
  }

  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar': {
        const storeRecordName = getParentRecordKey(field, variables);
        const value = storeRecord[storeRecordName];
        // TODO consider making scalars into discriminated unions. This probably has
        // to happen for when we handle errors.
        if (value === undefined) {
          return {
            kind: 'MissingData',
            reason: 'No value for ' + storeRecordName + ' on root ' + root,
          };
        }
        target[field.alias ?? field.fieldName] = value;
        break;
      }
      case 'Linked': {
        const storeRecordName = getParentRecordKey(field, variables);
        const value = storeRecord[storeRecordName];
        if (Array.isArray(value)) {
          const results = [];
          for (const item of value) {
            const link = assertLink(item);
            if (link === undefined) {
              return {
                kind: 'MissingData',
                reason:
                  'No link for ' +
                  storeRecordName +
                  ' on root ' +
                  root +
                  '. Link is ' +
                  JSON.stringify(item),
              };
            } else if (link === null) {
              results.push(null);
              continue;
            }
            const result = readData(
              environment,
              field.selections,
              link.__link,
              variables,
              nestedRefetchQueries,
            );
            if (result.kind === 'MissingData') {
              return {
                kind: 'MissingData',
                reason:
                  'Missing data for ' +
                  storeRecordName +
                  ' on root ' +
                  root +
                  '. Link is ' +
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
          const missingFieldHandler =
            environment.missingFieldHandler ?? defaultMissingFieldHandler;
          const altLink = missingFieldHandler(
            storeRecord,
            root,
            field.fieldName,
            field.arguments,
            variables,
          );
          if (altLink === undefined) {
            return {
              kind: 'MissingData',
              reason:
                'No link for ' +
                storeRecordName +
                ' on root ' +
                root +
                '. Link is ' +
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
          environment,
          field.selections,
          targetId,
          variables,
          nestedRefetchQueries,
        );
        if (data.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason: 'Missing data for ' + storeRecordName + ' on root ' + root,
            nestedReason: data,
          };
        }
        target[field.alias ?? field.fieldName] = data.data;
        break;
      }
      case 'RefetchField': {
        const data = readData(
          environment,
          field.readerArtifact.readerAst,
          root,
          variables,
          // Refetch fields just read the id, and don't need refetch query artifacts
          [],
        );
        if (typeof window !== 'undefined' && window.__LOG) {
          console.log('refetch field data', data, field);
        }
        if (data.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason: 'Missing data for ' + field.alias + ' on root ' + root,
            nestedReason: data,
          };
        } else {
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error('refetchQuery is null in RefetchField');
          }
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          target[field.alias] = field.readerArtifact.resolver(
            environment,
            // resolvers for refetch fields take 3 args, and this is not reflected in types
            refetchQueryArtifact,
            // @ts-expect-error
            {
              ...data.data,
              // TODO continue from here
              // variables need to be filtered for what we need just for the refetch query
              ...filterVariables(variables, allowedVariables),
            },
          );
        }
        break;
      }
      case 'MutationField': {
        const data = readData(
          environment,
          field.readerArtifact.readerAst,
          root,
          variables,
          // Refetch fields just read the id, and don't need refetch query artifacts
          [],
        );
        if (typeof window !== 'undefined' && window.__LOG) {
          console.log('refetch field data', data, field);
        }
        if (data.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason: 'Missing data for ' + field.alias + ' on root ' + root,
            nestedReason: data,
          };
        } else {
          const refetchQueryIndex = field.refetchQuery;
          if (refetchQueryIndex == null) {
            throw new Error('refetchQuery is null in MutationField');
          }
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          target[field.alias] = field.readerArtifact.resolver(
            environment,
            refetchQueryArtifact,
            // @ts-expect-error
            data.data,
            filterVariables(variables, allowedVariables),
          );
        }
        break;
      }
      case 'Resolver': {
        const usedRefetchQueries = field.usedRefetchQueries;
        const resolverRefetchQueries = usedRefetchQueries.map(
          (index) => nestedRefetchQueries[index],
        );

        const variant = field.readerArtifact.variant;
        if (variant.kind === 'Eager') {
          const data = readData(
            environment,
            field.readerArtifact.readerAst,
            root,
            variables,
            resolverRefetchQueries,
          );
          if (data.kind === 'MissingData') {
            return {
              kind: 'MissingData',
              reason: 'Missing data for ' + field.alias + ' on root ' + root,
              nestedReason: data,
            };
          } else {
            // @ts-expect-error
            target[field.alias] = field.readerArtifact.resolver(data.data);
          }
        } else if (variant.kind === 'Component') {
          target[field.alias] = getOrCreateCachedComponent(
            environment,
            root,
            variant.componentName,
            field.readerArtifact,
            variables,
            resolverRefetchQueries,
          );
        }
        break;
      }
    }
  }
  return { kind: 'Success', data: target as any };
}

export function defaultMissingFieldHandler(
  _storeRecord: StoreRecord,
  _root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null,
): Link | undefined {
  if (fieldName === 'node' || fieldName === 'user') {
    const variable = arguments_?.['id'];
    const value = variables?.[variable];

    // TODO can we handle explicit nulls here too? Probably, after wrapping in objects
    if (value != null) {
      return { __link: value };
    }
  }
}

export function assertLink(link: DataTypeValue): Link | null {
  if (Array.isArray(link)) {
    throw new Error('Unexpected array');
  }
  if (link == null) {
    return null;
  }
  if (typeof link === 'object') {
    return link;
  }
  throw new Error('Invalid link');
}

export type IsographComponentProps<TDataType, TOtherProps = Object> = {
  data: TDataType;
} & TOtherProps;

function filterVariables(
  variables: { [index: string]: string },
  allowedVariables: string[],
): { [index: string]: string } {
  const result: { [index: string]: string } = {};
  for (const key of allowedVariables) {
    result[key] = variables[key];
  }
  return result;
}
