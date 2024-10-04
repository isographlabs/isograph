import { CleanupFn } from '@isograph/isograph-disposable-types/dist';
import { getParentRecordKey, onNextChange } from './cache';
import { getOrCreateCachedComponent } from './componentCache';
import {
  IsographEntrypoint,
  RefetchQueryNormalizationArtifactWrapper,
} from './entrypoint';
import { FragmentReference, Variables } from './FragmentReference';
import {
  assertLink,
  DataId,
  defaultMissingFieldHandler,
  getOrLoadIsographArtifact,
  IsographEnvironment,
} from './IsographEnvironment';
import { makeNetworkRequest } from './makeNetworkRequest';
import {
  getPromiseState,
  PromiseWrapper,
  readPromise,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import { ReaderAst } from './reader';
import { Arguments } from './util';

export type WithEncounteredRecords<T> = {
  readonly encounteredRecords: Set<DataId>;
  readonly item: T;
};

export function readButDoNotEvaluate<
  TReadFromStore extends { parameters: object; data: object },
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): WithEncounteredRecords<TReadFromStore> {
  const mutableEncounteredRecords = new Set<DataId>();

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
  // @ts-expect-error
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('done reading', { response });
  }
  if (response.kind === 'MissingData') {
    // There are two cases here that we care about:
    // 1. the network request is in flight, we haven't suspend on it, and we want
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
      // TODO assert that the network request state is not Err
      throw new Promise((resolve, reject) => {
        onNextChange(environment).then(resolve);
        fragmentReference.networkRequest.promise.catch(reject);
      });
    }
    throw onNextChange(environment);
  } else {
    return {
      encounteredRecords: mutableEncounteredRecords,
      item: response.data,
    };
  }
}

type ReadDataResult<TReadFromStore> =
  | {
      readonly kind: 'Success';
      readonly data: TReadFromStore;
      readonly encounteredRecords: Set<DataId>;
    }
  | {
      readonly kind: 'MissingData';
      readonly reason: string;
      readonly nestedReason?: ReadDataResult<unknown>;
    };

function readData<TReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: DataId,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableEncounteredRecords: Set<DataId>,
): ReadDataResult<TReadFromStore> {
  mutableEncounteredRecords.add(root);
  let storeRecord = environment.store[root];
  if (storeRecord === undefined) {
    return {
      kind: 'MissingData',
      reason: 'No record for root ' + root,
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
          networkRequest,
          networkRequestOptions,
          mutableEncounteredRecords,
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

          // Second, we allow the user to call the resolver, which will ultimately
          // use the resolver reader AST to get the resolver parameters.
          target[field.alias] = (args: any) => [
            // Stable id
            root + '__' + field.name,
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
        const resolverRefetchQueries = usedRefetchQueries.map(
          (index) => nestedRefetchQueries[index],
        );

        switch (field.readerArtifact.kind) {
          case 'EagerReaderArtifact': {
            const data = readData(
              environment,
              field.readerArtifact.readerAst,
              root,
              variables,
              resolverRefetchQueries,
              networkRequest,
              networkRequestOptions,
              mutableEncounteredRecords,
            );
            if (data.kind === 'MissingData') {
              return {
                kind: 'MissingData',
                reason: 'Missing data for ' + field.alias + ' on root ' + root,
                nestedReason: data,
              };
            } else {
              const firstParameter = {
                data: data.data,
                parameters: variables,
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
            reason: 'Missing data for ' + field.alias + ' on root ' + root,
            nestedReason: refetchReaderParams,
          };
        } else {
          target[field.alias] = (args: any) => {
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
              root +
                '/' +
                field.name +
                '/' +
                stableStringifyArgs(localVariables),
              // Fetcher
              () => {
                const fragmentReferenceAndDisposeFromEntrypoint = (
                  entrypoint: IsographEntrypoint<any, any>,
                ): [FragmentReference<any, any>, CleanupFn] => {
                  const [networkRequest, disposeNetworkRequest] =
                    makeNetworkRequest(environment, entrypoint, localVariables);

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
                    root: localVariables.id,
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
                              makeNetworkRequest(
                                environment,
                                entrypoint,
                                localVariables,
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
                      root: localVariables.id,
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
      childVars[name] = variables[value.name];
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
function stableStringifyArgs(args: Object) {
  const keys = Object.keys(args);
  keys.sort();
  let s = '';
  for (const key of keys) {
    // @ts-expect-error
    s += `${key}=${JSON.stringify(args[key])};`;
  }
  return s;
}
