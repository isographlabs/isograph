import {
  callSubscriptions,
  getParentRecordKey,
  insertIfNotExists,
  type EncounteredIds,
} from './cache';
import type { RefetchQueryNormalizationArtifactWrapper } from './entrypoint';
import {
  stableIdForFragmentReference,
  type ExtractParameters,
  type ExtractStartUpdate,
  type ExtractUpdatableData,
  type FragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import {
  assertLink,
  type IsographEnvironment,
  type Link,
} from './IsographEnvironment';
import { readPromise, type PromiseWrapper } from './PromiseWrapper';
import {
  readLinkedFieldData,
  readScalarFieldData,
  type NetworkRequestReaderOptions,
  type ReadDataResultSuccess,
} from './read';
import type { ReaderAst } from './reader';

export function getOrCreateCachedStartUpdate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  eagerResolverName: string,
  networkRequestOptions: NetworkRequestReaderOptions,
): ExtractStartUpdate<TReadFromStore> {
  return (environment.eagerReaderCache[
    stableIdForFragmentReference(fragmentReference, eagerResolverName)
  ] ??= createStartUpdate(
    environment,
    fragmentReference,
    networkRequestOptions,
  ));
}

export function createStartUpdate<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
): ExtractStartUpdate<TReadFromStore> {
  return (updater) => {
    let mutableUpdatedIds: EncounteredIds = new Map();

    let data = createUpdatableProxy(
      environment,
      fragmentReference,
      networkRequestOptions,
      mutableUpdatedIds,
    );

    updater(data);
    callSubscriptions(environment, mutableUpdatedIds);
  };
}

export function createUpdatableProxy<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, unknown>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableUpdatedIds: EncounteredIds,
): ExtractUpdatableData<TReadFromStore> {
  const readerWithRefetchQueries = readPromise(
    fragmentReference.readerWithRefetchQueries,
  );

  return readUpdatableData(
    environment,
    readerWithRefetchQueries.readerArtifact.readerAst,
    fragmentReference.root,
    fragmentReference.variables ?? {},
    readerWithRefetchQueries.nestedRefetchQueries,
    fragmentReference.networkRequest,
    networkRequestOptions,
    {
      lastInvalidated: 0,
    },
    mutableUpdatedIds,
  ).data;
}

function defineCachedProperty<T>(
  target: T,
  property: PropertyKey,
  {
    get,
    set,
    mutableState,
  }: {
    mutableState: {
      lastInvalidated: number;
    };
    get(): any;
    set?(v: any): void;
  },
) {
  let value = get();
  let lastInvalidated = Date.now();
  Object.defineProperty(target, property, {
    configurable: true,
    enumerable: true,
    get: () => {
      if (lastInvalidated <= mutableState.lastInvalidated) {
        value = get();
        lastInvalidated = Date.now();
      }
      return value;
    },
    ...(set && {
      set: (newValue) => {
        set(newValue);
        mutableState.lastInvalidated = Date.now();
      },
    }),
  });
}

function readUpdatableData<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  ast: ReaderAst<TReadFromStore>,
  root: Link,
  variables: ExtractParameters<TReadFromStore>,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  networkRequest: PromiseWrapper<void, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  mutableState: {
    lastInvalidated: number;
  },
  mutableUpdatedIds: EncounteredIds,
): ReadDataResultSuccess<ExtractUpdatableData<TReadFromStore>> {
  let storeRecord = environment.store[root.__typename]?.[root.__link];
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
        const storeRecordName = getParentRecordKey(field, variables);

        defineCachedProperty(target, field.alias ?? field.fieldName, {
          mutableState,
          get: () => {
            const data = readScalarFieldData(
              field,
              storeRecord,
              root,
              variables,
            );
            if (data.kind === 'MissingData') {
              throw new Error(data.reason);
            }
            return data.data;
          },
          ...(field.isUpdatable
            ? {
                set: (newValue) => {
                  storeRecord[storeRecordName] = newValue;
                  const updatedIds = insertIfNotExists(
                    mutableUpdatedIds,
                    root.__typename,
                  );
                  updatedIds.add(root.__link);
                },
              }
            : undefined),
        });
        break;
      }
      case 'Linked': {
        const storeRecordName = getParentRecordKey(field, variables);
        defineCachedProperty(target, field.alias ?? field.fieldName, {
          mutableState,
          get: () => {
            const data = readLinkedFieldData(
              environment,
              field,
              storeRecord,
              root,
              variables,
              networkRequest,
              networkRequestOptions,
              (ast, root) =>
                readUpdatableData(
                  environment,
                  ast,
                  root,
                  variables,
                  nestedRefetchQueries,
                  networkRequest,
                  networkRequestOptions,
                  mutableState,
                  mutableUpdatedIds,
                ),
            );
            if (data.kind === 'MissingData') {
              throw new Error(data.reason);
            }
            return data.data;
          },
          ...('isUpdatable' in field && field.isUpdatable
            ? {
                set: (newValue) => {
                  if (Array.isArray(newValue)) {
                    storeRecord[storeRecordName] = newValue.map((node) =>
                      assertLink(node?.link),
                    );
                  } else {
                    storeRecord[storeRecordName] = assertLink(newValue?.link);
                  }
                  const updatedIds = insertIfNotExists(
                    mutableUpdatedIds,
                    root.__typename,
                  );
                  updatedIds.add(root.__link);
                },
              }
            : undefined),
        });
        break;
      }
      case 'ImperativelyLoadedField': {
        break;
      }
      case 'Resolver': {
        break;
      }
      case 'LoadablySelectedField': {
        break;
      }
      case 'Link': {
        target[field.alias] = root;
        break;
      }
      default: {
        field satisfies never;
        throw new Error('Unexpected case.');
      }
    }
  }

  return {
    kind: 'Success',
    data: target as any,
  };
}
