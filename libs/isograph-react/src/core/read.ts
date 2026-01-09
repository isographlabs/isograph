import type { CleanupFn, ItemCleanupPair } from '@isograph/disposable-types';
import {
  getParentRecordKey,
  insertEmptySetIfMissing,
  onNextChangeToRecord,
  type EncounteredIds,
} from './cache';
import type { FetchOptions } from './check';
import { getOrCreateCachedComponent } from './componentCache';
import type {
  IsographEntrypoint,
  ReaderWithRefetchQueries,
  RefetchQueryNormalizationArtifactWrapper,
} from './entrypoint';
import type {
  ExtractData,
  FragmentReference,
  UnknownTReadFromStore,
  Variables,
} from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';
import {
  assertLink,
  getOrLoadIsographArtifact,
  getOrLoadReaderWithRefetchQueries,
  isWithErrors,
  type DataTypeValue,
  type StoreLink,
  type StoreRecord,
} from './IsographEnvironment';
import { logMessage } from './logging';
import { maybeMakeNetworkRequest } from './makeNetworkRequest';
import { getStoreRecordProxy } from './optimisticProxy';
import type { PromiseWrapper } from './PromiseWrapper';
import {
  getPromiseState,
  NOT_SET,
  readPromise,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import type {
  LoadablySelectedField,
  ReaderAst,
  ReaderClientPointer,
  ReaderImperativelyLoadedField,
  ReaderLinkedField,
  ReaderNonLoadableResolverField,
  ReaderScalarField,
} from './reader';
import { getOrCreateCachedStartUpdate } from './startUpdate';
import type { Arguments } from './util';

export type WithEncounteredRecords<T> = {
  readonly encounteredRecords: EncounteredIds;
  readonly item: ExtractData<T>;
};

export function readButDoNotEvaluate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): WithEncounteredRecords<TReadFromStore> {
  const mutableEncounteredRecords: EncounteredIds = new Map();

  // TODO consider moving this to the outside
  const readerWithRefetchQueries = readPromise(
    fragmentReference.readerWithRefetchQueries,
  );

  const response = readData(
    environment,
    readerWithRefetchQueries.readerArtifact.readerAst,
    fragmentReference.root,
    fragmentReference.variables ?? {},
    readerWithRefetchQueries.nestedRefetchQueries,
    fragmentReference.networkRequest,
    networkRequestOptions,
    mutableEncounteredRecords,
  );

  logMessage(environment, () => ({
    kind: 'DoneReading',
    response,
    fieldName: readerWithRefetchQueries.readerArtifact.fieldName,
    root: fragmentReference.root,
  }));

  if (response.kind === 'MissingData') {
    // There are two cases here that we care about:
    // 1. the network request is in flight, we haven't suspended on it, and we want
    //    to throw if it errors out. So, networkRequestOptions.suspendIfInFlight === false
    //    and networkRequestOptions.throwOnNetworkError === true.
    // 2. everything else
    //
    // In the first case, we cannot simply throw onNextChange, because if the network
    // response errors out, we will not update the store, so the onNextChange promise
    // will not resolve.
    if (
      !networkRequestOptions.suspendIfInFlight &&
      networkRequestOptions.throwOnNetworkError
    ) {
      // What are we doing here? If the network response has errored out, we can do
      // two things: throw a rejected promise, or throw an error. Both work identically
      // in the browser. However, during initial SSR on NextJS, throwing a rejected
      // promise results in an infinite loop (including re-issuing the query until the
      // process OOM's or something.) Hence, we throw an error.

      const result = fragmentReference.networkRequest.result;
      if (result !== NOT_SET && result.kind === 'Err') {
        throw new Error('NetworkError', { cause: result.error });
      }

      throw new Promise((resolve, reject) => {
        onNextChangeToRecord(environment, response.recordLink).then(resolve);
        fragmentReference.networkRequest.promise.catch(reject);
      });
    }
    throw onNextChangeToRecord(environment, response.recordLink);
  } else {
    return {
      encounteredRecords: mutableEncounteredRecords,
      item: response.data,
    };
  }
}

export type ReadDataResultSuccess<Data> = {
  readonly kind: 'Success';
  readonly data: Data;
};

export type ReadDataResult<Data> =
  | ReadDataResultSuccess<Data>
  | {
      readonly kind: 'MissingData';
      readonly reason: string;
      readonly nestedReason?: ReadDataResult<unknown>;
      readonly recordLink: StoreLink;
    };

function readData<TReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: StoreLink,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableEncounteredRecords: EncounteredIds,
): ReadDataResult<ExtractData<TReadFromStore>> {
  const encounteredIds = insertEmptySetIfMissing(
    mutableEncounteredRecords,
    root.__typename,
  );
  encounteredIds.add(root.__link);
  let storeRecord = getStoreRecordProxy(environment.store, root);
  if (storeRecord === undefined) {
    return {
      kind: 'MissingData',
      reason: 'No record for root ' + root.__link,
      recordLink: root,
    };
  }

  if (storeRecord == null) {
    return {
      kind: 'Success',
      data: null as any,
    };
  }

  let target: { [index: string]: any } = {};

  for (const field of ast) {
    switch (field.kind) {
      case 'Scalar': {
        const data = readScalarFieldData(field, storeRecord, root, variables);

        if (data.kind === 'MissingData') {
          return data;
        }
        target[field.alias ?? field.fieldName] = data.data;
        break;
      }
      case 'Link': {
        target[field.alias] = root;
        break;
      }
      case 'Linked': {
        const data = readLinkedFieldData(
          environment,
          field,
          storeRecord,
          root,
          variables,
          nestedRefetchQueries,
          networkRequest,
          networkRequestOptions,
          (ast, root) =>
            readData(
              environment,
              ast,
              root,
              variables,
              nestedRefetchQueries,
              networkRequest,
              networkRequestOptions,
              mutableEncounteredRecords,
            ),
        );
        if (data.kind === 'MissingData') {
          return data;
        }
        target[field.alias ?? field.fieldName] = data.data;
        break;
      }
      case 'ImperativelyLoadedField': {
        const data = readImperativelyLoadedField(
          environment,
          field,
          root,
          variables,
          nestedRefetchQueries,
          networkRequest,
          networkRequestOptions,
          mutableEncounteredRecords,
        );
        if (data.kind === 'MissingData') {
          return data;
        }
        target[field.alias] = data.data;
        break;
      }
      case 'Resolver': {
        const data = readResolverFieldData(
          environment,
          field,
          root,
          variables,
          nestedRefetchQueries,
          networkRequest,
          networkRequestOptions,
          mutableEncounteredRecords,
        );
        if (data.kind === 'MissingData') {
          return data;
        }
        target[field.alias] = data.data;
        break;
      }
      case 'LoadablySelectedField': {
        const data = readLoadablySelectedFieldData(
          environment,
          field,
          root,
          variables,
          networkRequest,
          networkRequestOptions,
          mutableEncounteredRecords,
        );
        if (data.kind === 'MissingData') {
          return data;
        }
        target[field.alias] = data.data;
        break;
      }
    }
  }
  return {
    kind: 'Success',
    data: target as any,
  };
}

export function readLoadablySelectedFieldData(
  environment: IsographEnvironment,
  field: LoadablySelectedField,
  root: StoreLink,
  variables: Variables,
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableEncounteredRecords: EncounteredIds,
): ReadDataResult<unknown> {
  const refetchReaderParams = readData(
    environment,
    field.refetchReaderAst,
    root,
    variables,
    // Refetch fields just read the id, and don't need refetch query artifacts
    [],
    networkRequest,
    networkRequestOptions,
    mutableEncounteredRecords,
  );

  if (refetchReaderParams.kind === 'MissingData') {
    return {
      kind: 'MissingData',
      reason: 'Missing data for ' + field.alias + ' on root ' + root.__link,
      nestedReason: refetchReaderParams,
      recordLink: refetchReaderParams.recordLink,
    };
  }

  return {
    kind: 'Success',
    data: (
      args: any,
      // TODO get the associated type for FetchOptions from the loadably selected field
      fetchOptions?: FetchOptions<any, never>,
    ) => {
      // TODO we should use the reader AST for this
      const includeReadOutData = (variables: any, readOutData: any) => {
        variables.id = readOutData.id;
        return variables;
      };
      const localVariables = includeReadOutData(
        args ?? {},
        refetchReaderParams.data,
      );
      writeQueryArgsToVariables(
        localVariables,
        field.queryArguments,
        variables,
      );

      return [
        // Stable id
        root.__typename +
          ':' +
          root.__link +
          '/' +
          field.name +
          '/' +
          stableStringifyArgs(localVariables),
        // Fetcher
        () => {
          const fragmentReferenceAndDisposeFromEntrypoint = (
            entrypoint: IsographEntrypoint<any, any, any, {}>,
          ): [FragmentReference<any, any>, CleanupFn] => {
            const { fieldName, readerArtifactKind, readerWithRefetchQueries } =
              getOrLoadReaderWithRefetchQueries(
                environment,
                entrypoint.readerWithRefetchQueries,
              );
            const [networkRequest, disposeNetworkRequest] =
              maybeMakeNetworkRequest(
                environment,
                entrypoint,
                localVariables,
                readerWithRefetchQueries,
                fetchOptions ?? null,
              );

            const fragmentReference: FragmentReference<any, any> = {
              kind: 'FragmentReference',
              readerWithRefetchQueries,
              fieldName,
              readerArtifactKind,
              // TODO localVariables is not guaranteed to have an id field
              root,
              variables: localVariables,
              networkRequest,
            };
            return [fragmentReference, disposeNetworkRequest];
          };

          if (field.entrypoint.kind === 'Entrypoint') {
            return fragmentReferenceAndDisposeFromEntrypoint(field.entrypoint);
          } else {
            const isographArtifactPromiseWrapper = getOrLoadIsographArtifact(
              environment,
              field.entrypoint.typeAndField,
              field.entrypoint.loader,
            );
            const state = getPromiseState(isographArtifactPromiseWrapper);
            if (state.kind === 'Ok') {
              return fragmentReferenceAndDisposeFromEntrypoint(state.value);
            } else {
              // Promise is pending or thrown

              let entrypointLoaderState:
                | {
                    kind: 'EntrypointNotLoaded';
                  }
                | {
                    kind: 'NetworkRequestStarted';
                    disposeNetworkRequest: CleanupFn;
                  }
                | { kind: 'Disposed' } = { kind: 'EntrypointNotLoaded' };

              const readerWithRefetchQueries = wrapPromise(
                isographArtifactPromiseWrapper.promise.then(
                  (entrypoint) =>
                    getOrLoadReaderWithRefetchQueries(
                      environment,
                      entrypoint.readerWithRefetchQueries,
                    ).readerWithRefetchQueries.promise,
                ),
              );
              const networkRequest = wrapPromise(
                isographArtifactPromiseWrapper.promise.then((entrypoint) => {
                  if (entrypointLoaderState.kind === 'EntrypointNotLoaded') {
                    const [networkRequest, disposeNetworkRequest] =
                      maybeMakeNetworkRequest(
                        environment,
                        entrypoint,
                        localVariables,
                        readerWithRefetchQueries,
                        fetchOptions ?? null,
                      );
                    entrypointLoaderState = {
                      kind: 'NetworkRequestStarted',
                      disposeNetworkRequest,
                    };
                    return networkRequest.promise;
                  }
                }),
              );

              const fragmentReference: FragmentReference<any, any> = {
                kind: 'FragmentReference',
                readerWithRefetchQueries,
                fieldName: field.name,
                readerArtifactKind: field.entrypoint.readerArtifactKind,
                // TODO localVariables is not guaranteed to have an id field
                root,
                variables: localVariables,
                networkRequest,
              };

              return [
                fragmentReference,
                () => {
                  if (entrypointLoaderState.kind === 'NetworkRequestStarted') {
                    entrypointLoaderState.disposeNetworkRequest();
                  }
                  entrypointLoaderState = { kind: 'Disposed' };
                },
              ];
            }
          }
        },
      ];
    },
  };
}

function filterVariables(
  variables: Variables,
  allowedVariables: string[],
): Variables {
  const result: Variables = {};
  for (const key of allowedVariables) {
    // @ts-expect-error
    result[key] = variables[key];
  }
  return result;
}

function generateChildVariableMap(
  variables: Variables,
  fieldArguments: Arguments | null,
): Variables {
  if (fieldArguments == null) {
    return {};
  }

  type Writable<T> = { -readonly [P in keyof T]: T[P] };
  const childVars: Writable<Variables> = {};
  for (const [name, value] of fieldArguments) {
    if (value.kind === 'Object') {
      childVars[name] = generateChildVariableMap(variables, value.value);
    } else if (value.kind === 'Variable') {
      const variable = variables[value.name];
      // Variable could be null if it was not provided but has a default case,
      // so we allow the loop to continue rather than throwing an error.
      if (variable != null) {
        childVars[name] = variable;
      }
    } else {
      childVars[name] = value.value;
    }
  }
  return childVars;
}

function writeQueryArgsToVariables(
  targetVariables: any,
  queryArgs: Arguments | null,
  variables: Variables,
) {
  if (queryArgs == null) {
    return;
  }
  for (const [name, argType] of queryArgs) {
    switch (argType.kind) {
      case 'Object': {
        writeQueryArgsToVariables(
          (targetVariables[name] = {}),
          argType.value,
          variables,
        );
        break;
      }
      case 'Variable': {
        targetVariables[name] = variables[argType.name];
        break;
      }
      case 'Enum': {
        targetVariables[name] = argType.value;
        break;
      }
      case 'Literal': {
        targetVariables[name] = argType.value;
        break;
      }
      case 'String': {
        targetVariables[name] = argType.value;
        break;
      }
    }
  }
}

export function readResolverFieldData(
  environment: IsographEnvironment,
  field: ReaderNonLoadableResolverField,
  root: StoreLink,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableEncounteredRecords: EncounteredIds,
): ReadDataResult<unknown> {
  const usedRefetchQueries = field.usedRefetchQueries;
  const resolverRefetchQueries = usedRefetchQueries.map((index) => {
    const resolverRefetchQuery = nestedRefetchQueries[index];
    if (resolverRefetchQuery == null) {
      throw new Error(
        'resolverRefetchQuery is null in Resolver. This is indicative of a bug in Isograph.',
      );
    }
    return resolverRefetchQuery;
  });

  const readerWithRefetchQueries = {
    kind: 'ReaderWithRefetchQueries',
    readerArtifact: field.readerArtifact,
    nestedRefetchQueries: resolverRefetchQueries,
  } satisfies ReaderWithRefetchQueries<any, any>;

  const fragment = {
    kind: 'FragmentReference',
    readerWithRefetchQueries: wrapResolvedValue(readerWithRefetchQueries),
    fieldName: field.readerArtifact.fieldName,
    readerArtifactKind: field.readerArtifact.kind,
    root,
    variables: generateChildVariableMap(variables, field.arguments),
    networkRequest,
  } satisfies FragmentReference<any, any>;

  switch (field.readerArtifact.kind) {
    case 'EagerReaderArtifact': {
      const data = readData(
        environment,
        field.readerArtifact.readerAst,
        root,
        generateChildVariableMap(variables, field.arguments),
        resolverRefetchQueries,
        networkRequest,
        networkRequestOptions,
        mutableEncounteredRecords,
      );
      if (data.kind === 'MissingData') {
        return {
          kind: 'MissingData',
          reason: 'Missing data for ' + field.alias + ' on root ' + root.__link,
          nestedReason: data,
          recordLink: data.recordLink,
        };
      }
      const firstParameter = {
        data: data.data,
        parameters: variables,
        startUpdate: field.readerArtifact.hasUpdatable
          ? getOrCreateCachedStartUpdate(
              environment,
              fragment,
              networkRequestOptions,
            )
          : undefined,
      };
      return {
        kind: 'Success',
        data: field.readerArtifact.resolver(firstParameter),
      };
    }
    case 'ComponentReaderArtifact': {
      return {
        kind: 'Success',
        data: getOrCreateCachedComponent(
          environment,
          fragment,
          networkRequestOptions,
        ),
      };
    }
  }
}

export function readScalarFieldData(
  field: ReaderScalarField,
  storeRecord: StoreRecord,
  root: StoreLink,
  variables: Variables,
): ReadDataResult<
  string | number | boolean | StoreLink | readonly DataTypeValue[] | null
> {
  const storeRecordName = getParentRecordKey(field, variables);
  let value = storeRecord[storeRecordName];
  // TODO consider making scalars into discriminated unions. This probably has
  // to happen for when we handle errors.
  if (value === undefined) {
    return {
      kind: 'MissingData',
      reason: 'No value for ' + storeRecordName + ' on root ' + root.__link,
      recordLink: root,
    };
  }

  if (isWithErrors(value, field.isFallible ?? false)) {
    if (value.kind === 'Errors') {
      throw new Error('TODO: read errors');
    }
    value = value.value;
  }

  return { kind: 'Success', data: value };
}

export function readLinkedFieldData(
  environment: IsographEnvironment,
  field: ReaderLinkedField,
  storeRecord: StoreRecord,
  root: StoreLink,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  readData: <TReadFromStore>(
    ast: ReaderAst<TReadFromStore>,
    root: StoreLink,
  ) => ReadDataResult<object>,
): ReadDataResult<unknown> {
  const storeRecordName = getParentRecordKey(field, variables);
  let value = storeRecord[storeRecordName];

  if (isWithErrors(value, field.isFallible ?? false)) {
    if (value?.kind === 'Errors') {
      throw new Error('TODO: read errors');
    }
    value = value?.value;
  }

  if (field.condition != null) {
    const data = readData(field.condition.readerAst, root);
    if (data.kind === 'MissingData') {
      return {
        kind: 'MissingData',
        reason:
          'Missing data for ' + storeRecordName + ' on root ' + root.__link,
        nestedReason: data,
        recordLink: data.recordLink,
      };
    }

    const readerWithRefetchQueries = {
      kind: 'ReaderWithRefetchQueries',
      readerArtifact: field.condition,
      // TODO this is wrong
      // should map field.condition.usedRefetchQueries
      // but it doesn't exist
      nestedRefetchQueries: [],
    } satisfies ReaderWithRefetchQueries<any, any>;

    const fragment = {
      kind: 'FragmentReference',
      readerWithRefetchQueries: wrapResolvedValue(readerWithRefetchQueries),
      root,
      fieldName: field.condition.fieldName,
      readerArtifactKind: field.condition.kind,
      variables: generateChildVariableMap(
        variables,
        // TODO this is wrong
        // should use field.arguments
        // but it doesn't exist
        [],
      ),
      networkRequest,
    } satisfies FragmentReference<any, any>;

    const condition = field.condition.resolver({
      data: data.data,
      parameters: {},
      ...(field.condition.hasUpdatable
        ? {
            startUpdate: getOrCreateCachedStartUpdate(
              environment,
              fragment,
              networkRequestOptions,
            ),
          }
        : undefined),
    });
    value = condition;
  }

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
            root.__link +
            '. Link is ' +
            JSON.stringify(item),
          recordLink: root,
        };
      } else if (link == null) {
        results.push(null);
        continue;
      }

      if (isClientPointer(field)) {
        const result = readClientPointerData(
          environment,
          field,
          link,
          variables,
          nestedRefetchQueries,
          readData,
        );
        if (result.kind === 'MissingData') {
          return {
            kind: 'MissingData',
            reason:
              'Missing data for ' +
              storeRecordName +
              ' on root ' +
              root.__link +
              '. Link is ' +
              JSON.stringify(item),
            nestedReason: result,
            recordLink: result.recordLink,
          };
        }
        results.push(result.data);
        continue;
      }

      const result = readData(field.selections, link);
      if (result.kind === 'MissingData') {
        return {
          kind: 'MissingData',
          reason:
            'Missing data for ' +
            storeRecordName +
            ' on root ' +
            root.__link +
            '. Link is ' +
            JSON.stringify(item),
          nestedReason: result,
          recordLink: result.recordLink,
        };
      }
      results.push(result.data);
    }
    return {
      kind: 'Success',
      data: results,
    };
  }
  let link = assertLink(value);

  if (link === undefined) {
    // TODO make this configurable, and also generated and derived from the schema
    const missingFieldHandler = environment.missingFieldHandler;

    const altLink = missingFieldHandler?.(
      storeRecord,
      root,
      field.fieldName,
      field.arguments,
      variables,
    );
    logMessage(environment, () => ({
      kind: 'MissingFieldHandlerCalled',
      root,
      storeRecord,
      fieldName: field.fieldName,
      arguments: field.arguments,
      variables,
    }));

    if (altLink === undefined) {
      return {
        kind: 'MissingData',
        reason:
          'No link for ' +
          storeRecordName +
          ' on root ' +
          root.__link +
          '. Link is ' +
          JSON.stringify(value),
        recordLink: root,
      };
    } else {
      link = altLink;
    }
  } else if (link == null) {
    return {
      kind: 'Success',
      data: null,
    };
  }

  if (isClientPointer(field)) {
    const data = readClientPointerData(
      environment,
      field,
      link,
      variables,
      nestedRefetchQueries,
      readData,
    );
    if (data.kind === 'MissingData') {
      return {
        kind: 'MissingData',
        reason:
          'Missing data for ' + storeRecordName + ' on root ' + root.__link,
        nestedReason: data,
        recordLink: data.recordLink,
      };
    }
    return data;
  }
  const data = readData(field.selections, link);
  if (data.kind === 'MissingData') {
    return {
      kind: 'MissingData',
      reason: 'Missing data for ' + storeRecordName + ' on root ' + root.__link,
      nestedReason: data,
      recordLink: data.recordLink,
    };
  }
  return data;
}

function isClientPointer(
  field: ReaderLinkedField,
): field is ReaderClientPointer {
  return field.refetchQueryIndex != null;
}

export function readClientPointerData(
  environment: IsographEnvironment,
  field: ReaderClientPointer,
  root: StoreLink,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  readData: <TReadFromStore>(
    ast: ReaderAst<TReadFromStore>,
    root: StoreLink,
  ) => ReadDataResult<object>,
): ReadDataResult<unknown> {
  const refetchReaderParams = readData(
    [
      {
        kind: 'Scalar',
        fieldName: 'id',
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
    root,
  );

  if (refetchReaderParams.kind === 'MissingData') {
    return {
      kind: 'MissingData',
      reason: 'Missing data for ' + field.alias + ' on root ' + root.__link,
      nestedReason: refetchReaderParams,
      recordLink: refetchReaderParams.recordLink,
    };
  }

  const refetchQuery = nestedRefetchQueries[field.refetchQueryIndex];
  if (refetchQuery == null) {
    throw new Error(
      'refetchQuery is null in RefetchField. This is indicative of a bug in Isograph.',
    );
  }
  const refetchQueryArtifact = refetchQuery.artifact;
  const allowedVariables = refetchQuery.allowedVariables;

  return {
    kind: 'Success',
    data: (
      args: any,
      // TODO get the associated type for FetchOptions from the loadably selected field
      fetchOptions?: FetchOptions<any, never>,
    ) => {
      const includeReadOutData = (variables: any, readOutData: any) => {
        variables.id = readOutData.id;
        return variables;
      };
      const localVariables = includeReadOutData(
        args ?? {},
        refetchReaderParams.data,
      );
      writeQueryArgsToVariables(localVariables, field.arguments, variables);

      return [
        // Stable id
        root.__typename +
          ':' +
          root.__link +
          '/' +
          field.fieldName +
          '/' +
          stableStringifyArgs(localVariables),
        // Fetcher
        (): ItemCleanupPair<FragmentReference<any, any>> | undefined => {
          const variables = includeReadOutData(
            filterVariables({ ...args, ...localVariables }, allowedVariables),
            refetchReaderParams.data,
          );

          const readerWithRefetchQueries = wrapResolvedValue({
            kind: 'ReaderWithRefetchQueries',
            readerArtifact: {
              kind: 'EagerReaderArtifact',
              fieldName: field.fieldName,
              readerAst: field.selections,
              resolver: ({ data }: { data: any }) => data,
              hasUpdatable: false,
            },
            nestedRefetchQueries,
          } as const);

          const [networkRequest, disposeNetworkRequest] =
            maybeMakeNetworkRequest(
              environment,
              refetchQueryArtifact,
              variables,
              readerWithRefetchQueries,
              fetchOptions ?? null,
            );

          const fragmentReference: FragmentReference<any, any> = {
            kind: 'FragmentReference',
            fieldName: field.fieldName,
            readerArtifactKind: 'EagerReaderArtifact',
            readerWithRefetchQueries: readerWithRefetchQueries,
            root,
            variables,
            networkRequest,
          };
          return [fragmentReference, disposeNetworkRequest];
        },
      ];
    },
  };
}

export type NetworkRequestReaderOptions = {
  suspendIfInFlight: boolean;
  throwOnNetworkError: boolean;
};

export function getNetworkRequestOptionsWithDefaults(
  networkRequestOptions?: Partial<NetworkRequestReaderOptions> | void,
): NetworkRequestReaderOptions {
  return {
    suspendIfInFlight: networkRequestOptions?.suspendIfInFlight ?? false,
    throwOnNetworkError: networkRequestOptions?.throwOnNetworkError ?? true,
  };
}

// TODO use a description of the params for this?
// TODO call stableStringifyArgs on the variable values, as well.
// This doesn't matter for now, since we are just using primitive values
// in the demo.
function stableStringifyArgs(args: object) {
  const keys = Object.keys(args);
  keys.sort();
  let s = '';
  for (const key of keys) {
    // @ts-expect-error
    s += `${key}=${JSON.stringify(args[key])};`;
  }
  return s;
}

export function readImperativelyLoadedField(
  environment: IsographEnvironment,
  field: ReaderImperativelyLoadedField,
  root: StoreLink,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableEncounteredRecords: EncounteredIds,
): ReadDataResult<unknown> {
  // First, we read the data using the refetch reader AST (i.e. read out the
  // id field).
  const data = readData(
    environment,
    field.refetchReaderArtifact.readerAst,
    root,
    variables,
    // Refetch fields just read the id, and don't need refetch query artifacts
    [],
    // This is probably indicative of the fact that we are doing redundant checks
    // on the status of this network request...
    networkRequest,
    networkRequestOptions,
    mutableEncounteredRecords,
  );
  if (data.kind === 'MissingData') {
    return {
      kind: 'MissingData',
      reason: 'Missing data for ' + field.alias + ' on root ' + root.__link,
      nestedReason: data,
      recordLink: data.recordLink,
    };
  } else {
    const { refetchQueryIndex } = field;
    const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
    if (refetchQuery == null) {
      throw new Error(
        'Refetch query not found. This is indicative of a bug in Isograph.',
      );
    }
    const refetchQueryArtifact = refetchQuery.artifact;
    const allowedVariables = refetchQuery.allowedVariables;

    // Second, we allow the user to call the resolver, which will ultimately
    // use the resolver reader AST to get the resolver parameters.
    return {
      kind: 'Success',
      data: (args: any) => [
        // Stable id
        root.__typename + ':' + root.__link + '__' + field.name,
        // Fetcher
        field.refetchReaderArtifact.resolver(
          environment,
          refetchQueryArtifact,
          data.data,
          filterVariables({ ...args, ...variables }, allowedVariables),
          root,
          // TODO these params should be removed
          null,
          [],
        ),
      ],
    };
  }
}
