import { CleanupFn } from '@isograph/disposable-types';
import {
  getParentRecordKey,
  insertIfNotExists,
  onNextChangeToRecord,
  type EncounteredIds,
} from './cache';
import { FetchOptions } from './check';
import { getOrCreateCachedComponent } from './componentCache';
import {
  IsographEntrypoint,
  RefetchQueryNormalizationArtifactWrapper,
} from './entrypoint';
import {
  ExtractData,
  ExtractParameters,
  FragmentReference,
  Variables,
  type UnknownTReadFromStore,
} from './FragmentReference';
import {
  assertLink,
  getOrLoadIsographArtifact,
  IsographEnvironment,
  type Link,
} from './IsographEnvironment';
import { logMessage } from './logging';
import { maybeMakeNetworkRequest } from './makeNetworkRequest';
import {
  getPromiseState,
  PromiseWrapper,
  readPromise,
  Result,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import { ReaderAst } from './reader';
import { startUpdate } from './startUpdate';
import { Arguments } from './util';

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

  logMessage(environment, {
    kind: 'DoneReading',
    response,
  });

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

      // TODO investigate why we cannot check against NOT_SET here and we have to cast
      const result = fragmentReference.networkRequest.result as Result<
        any,
        any
      >;
      if (result.kind === 'Err') {
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

export type ReadDataResult<TReadFromStore> =
  | {
      readonly kind: 'Success';
      readonly data: ExtractData<TReadFromStore>;
      readonly encounteredRecords: EncounteredIds;
    }
  | {
      readonly kind: 'MissingData';
      readonly reason: string;
      readonly nestedReason?: ReadDataResult<unknown>;
      readonly recordLink: Link;
    };

function readData<TReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: Link,
  variables: ExtractParameters<TReadFromStore>,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableEncounteredRecords: EncounteredIds,
): ReadDataResult<TReadFromStore> {
  const encounteredIds = insertIfNotExists(
    mutableEncounteredRecords,
    root.__typename,
  );
  encounteredIds.add(root.__link);
  let storeRecord = environment.store[root.__typename]?.[root.__link];
  if (storeRecord === undefined) {
    return {
      kind: 'MissingData',
      reason: 'No record for root ' + root.__link,
      recordLink: root,
    };
  }

  if (storeRecord === null) {
    return {
      kind: 'Success',
      data: null as any,
      encounteredRecords: mutableEncounteredRecords,
    };
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
            reason:
              'No value for ' + storeRecordName + ' on root ' + root.__link,
            recordLink: root,
          };
        }
        target[field.alias ?? field.fieldName] = value;
        break;
      }
      case 'Link': {
        target[field.alias] = root;
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
                  root.__link +
                  '. Link is ' +
                  JSON.stringify(item),
                recordLink: root,
              };
            } else if (link === null) {
              results.push(null);
              continue;
            }

            const result = readData(
              environment,
              field.selections,
              link,
              variables,
              nestedRefetchQueries,
              networkRequest,
              networkRequestOptions,
              mutableEncounteredRecords,
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
          }
          target[field.alias ?? field.fieldName] = results;
          break;
        }
        let link = assertLink(value);

        if (field.condition) {
          const data = readData(
            environment,
            field.condition.readerAst,
            root,
            variables,
            nestedRefetchQueries,
            networkRequest,
            networkRequestOptions,
            mutableEncounteredRecords,
          );
          if (data.kind === 'MissingData') {
            return {
              kind: 'MissingData',
              reason:
                'Missing data for ' +
                storeRecordName +
                ' on root ' +
                root.__link,
              nestedReason: data,
              recordLink: data.recordLink,
            };
          }
          const condition = field.condition.resolver({
            data: data.data,
            parameters: {},
            startUpdate: field.condition.hasUpdatable
              ? startUpdate(environment, data)
              : undefined,
          });
          if (condition === true) {
            link = root;
          } else if (condition === false) {
            link = null;
          } else {
            link = condition;
          }
        }

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
          logMessage(environment, {
            kind: 'MissingFieldHandlerCalled',
            root,
            storeRecord,
            fieldName: field.fieldName,
            arguments: field.arguments,
            variables,
          });

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
        } else if (link === null) {
          target[field.alias ?? field.fieldName] = null;
          break;
        }
        const targetId = link;
        const data = readData(
          environment,
          field.selections,
          targetId,
          variables,
          nestedRefetchQueries,
          networkRequest,
          networkRequestOptions,
          mutableEncounteredRecords,
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
        target[field.alias ?? field.fieldName] = data.data;
        break;
      }
      case 'ImperativelyLoadedField': {
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
            reason:
              'Missing data for ' + field.alias + ' on root ' + root.__link,
            nestedReason: data,
            recordLink: data.recordLink,
          };
        } else {
          const refetchQueryIndex = field.refetchQuery;
          const refetchQuery = nestedRefetchQueries[refetchQueryIndex];
          if (refetchQuery == null) {
            throw new Error(
              'refetchQuery is null in RefetchField. This is indicative of a bug in Isograph.',
            );
          }
          const refetchQueryArtifact = refetchQuery.artifact;
          const allowedVariables = refetchQuery.allowedVariables;

          // Second, we allow the user to call the resolver, which will ultimately
          // use the resolver reader AST to get the resolver parameters.
          target[field.alias] = (args: any) => [
            // Stable id
            root.__link + '__' + field.name,
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
          ];
        }
        break;
      }
      case 'Resolver': {
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
                reason:
                  'Missing data for ' + field.alias + ' on root ' + root.__link,
                nestedReason: data,
                recordLink: data.recordLink,
              };
            } else {
              const firstParameter = {
                data: data.data,
                parameters: variables,
                startUpdate: () => {},
              };
              target[field.alias] =
                field.readerArtifact.resolver(firstParameter);
            }
            break;
          }
          case 'ComponentReaderArtifact': {
            target[field.alias] = getOrCreateCachedComponent(
              environment,
              field.readerArtifact.componentName,
              {
                kind: 'FragmentReference',
                readerWithRefetchQueries: wrapResolvedValue({
                  kind: 'ReaderWithRefetchQueries',
                  readerArtifact: field.readerArtifact,
                  nestedRefetchQueries: resolverRefetchQueries,
                }),
                root,
                variables: generateChildVariableMap(variables, field.arguments),
                networkRequest,
              } as const,
              networkRequestOptions,
            );
            break;
          }
          default: {
            let _: never = field.readerArtifact;
            _;
            throw new Error('Unexpected kind');
          }
        }
        break;
      }
      case 'LoadablySelectedField': {
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
            reason:
              'Missing data for ' + field.alias + ' on root ' + root.__link,
            nestedReason: refetchReaderParams,
            recordLink: refetchReaderParams.recordLink,
          };
        } else {
          target[field.alias] = (
            args: any,
            // TODO get the associated type for FetchOptions from the loadably selected field
            fetchOptions?: FetchOptions<any>,
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
                  entrypoint: IsographEntrypoint<any, any, any>,
                ): [FragmentReference<any, any>, CleanupFn] => {
                  const [networkRequest, disposeNetworkRequest] =
                    maybeMakeNetworkRequest(
                      environment,
                      entrypoint,
                      localVariables,
                      fetchOptions,
                    );

                  const fragmentReference: FragmentReference<any, any> = {
                    kind: 'FragmentReference',
                    readerWithRefetchQueries: wrapResolvedValue({
                      kind: 'ReaderWithRefetchQueries',
                      readerArtifact:
                        entrypoint.readerWithRefetchQueries.readerArtifact,
                      nestedRefetchQueries:
                        entrypoint.readerWithRefetchQueries
                          .nestedRefetchQueries,
                    } as const),

                    // TODO localVariables is not guaranteed to have an id field
                    root,
                    variables: localVariables,
                    networkRequest,
                  };
                  return [fragmentReference, disposeNetworkRequest];
                };

                if (field.entrypoint.kind === 'Entrypoint') {
                  return fragmentReferenceAndDisposeFromEntrypoint(
                    field.entrypoint,
                  );
                } else {
                  const isographArtifactPromiseWrapper =
                    getOrLoadIsographArtifact(
                      environment,
                      field.entrypoint.typeAndField,
                      field.entrypoint.loader,
                    );
                  const state = getPromiseState(isographArtifactPromiseWrapper);
                  if (state.kind === 'Ok') {
                    return fragmentReferenceAndDisposeFromEntrypoint(
                      state.value,
                    );
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

                    const networkRequest = wrapPromise(
                      isographArtifactPromiseWrapper.promise.then(
                        (entrypoint) => {
                          if (
                            entrypointLoaderState.kind === 'EntrypointNotLoaded'
                          ) {
                            const [networkRequest, disposeNetworkRequest] =
                              maybeMakeNetworkRequest(
                                environment,
                                entrypoint,
                                localVariables,
                                fetchOptions,
                              );
                            entrypointLoaderState = {
                              kind: 'NetworkRequestStarted',
                              disposeNetworkRequest,
                            };
                            return networkRequest.promise;
                          }
                        },
                      ),
                    );
                    const readerWithRefetchPromise =
                      isographArtifactPromiseWrapper.promise.then(
                        (entrypoint) => entrypoint.readerWithRefetchQueries,
                      );

                    const fragmentReference: FragmentReference<any, any> = {
                      kind: 'FragmentReference',
                      readerWithRefetchQueries: wrapPromise(
                        readerWithRefetchPromise,
                      ),

                      // TODO localVariables is not guaranteed to have an id field
                      root,
                      variables: localVariables,
                      networkRequest,
                    };

                    return [
                      fragmentReference,
                      () => {
                        if (
                          entrypointLoaderState.kind === 'NetworkRequestStarted'
                        ) {
                          entrypointLoaderState.disposeNetworkRequest();
                        }
                        entrypointLoaderState = { kind: 'Disposed' };
                      },
                    ];
                  }
                }
              },
            ];
          };
        }
        break;
      }

      default: {
        // Ensure we have covered all variants
        let _: never = field;
        _;
        throw new Error('Unexpected case.');
      }
    }
  }
  return {
    kind: 'Success',
    data: target as any,
    encounteredRecords: mutableEncounteredRecords,
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
    if (value.kind === 'Variable') {
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
      default: {
        const _: never = argType;
        _;
        throw new Error('Unexpected case');
      }
    }
  }
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
