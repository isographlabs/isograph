import {
  Factory,
  ItemCleanupPair,
  ParentCache,
} from '@isograph/react-disposable-state';
import {
  IsographEntrypoint,
  NormalizationInlineFragment,
  NormalizationLinkedField,
  NormalizationScalarField,
  RefetchQueryNormalizationArtifactWrapper,
  type NormalizationAstNodes,
} from '../core/entrypoint';
import { mergeObjectsUsingReaderAst } from './areEqualWithDeepComparison';
import { FetchOptions } from './check';
import {
  ExtractParameters,
  FragmentReference,
  Variables,
} from './FragmentReference';
import {
  DataId,
  DataTypeValue,
  FragmentSubscription,
  getLink,
  Link,
  ROOT_ID,
  StoreRecord,
  type IsographEnvironment,
  type TypeName,
} from './IsographEnvironment';
import { logMessage } from './logging';
import { maybeMakeNetworkRequest } from './makeNetworkRequest';
import { wrapResolvedValue } from './PromiseWrapper';
import { readButDoNotEvaluate, WithEncounteredRecords } from './read';
import {
  ReaderLinkedField,
  ReaderScalarField,
  type ReaderAst,
  type StartUpdate,
} from './reader';
import { Argument, ArgumentValue } from './util';

export const TYPENAME_FIELD_NAME = '__typename';

export function getOrCreateItemInSuspenseCache<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  index: string,
  factory: Factory<FragmentReference<TReadFromStore, TClientFieldValue>>,
): ParentCache<FragmentReference<TReadFromStore, TClientFieldValue>> {
  // TODO this is probably a useless message, we should remove it
  logMessage(environment, {
    kind: 'GettingSuspenseCacheItem',
    index,
    availableCacheItems: Object.keys(environment.fragmentCache),
    found: !!environment.fragmentCache[index],
  });
  if (environment.fragmentCache[index] == null) {
    environment.fragmentCache[index] = new ParentCache(factory);
  }

  return environment.fragmentCache[index];
}

/**
 * Creates a copy of the provided value, ensuring any nested objects have their
 * keys sorted such that equivalent values would have identical JSON.stringify
 * results.
 */
export function stableCopy<T>(value: T): T {
  if (!value || typeof value !== 'object') {
    return value;
  }
  if (Array.isArray(value)) {
    // @ts-ignore
    return value.map(stableCopy);
  }
  const keys = Object.keys(value).sort();
  const stable: { [index: string]: any } = {};
  for (let i = 0; i < keys.length; i++) {
    // @ts-ignore
    stable[keys[i]] = stableCopy(value[keys[i]]);
  }
  return stable as any;
}

export function getOrCreateCacheForArtifact<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): ParentCache<FragmentReference<TReadFromStore, TClientFieldValue>> {
  const cacheKey =
    entrypoint.networkRequestInfo.queryText +
    JSON.stringify(stableCopy(variables));
  const factory = () => {
    const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
      environment,
      entrypoint,
      variables,
      fetchOptions,
    );

    const itemCleanupPair: ItemCleanupPair<
      FragmentReference<TReadFromStore, TClientFieldValue>
    > = [
      {
        kind: 'FragmentReference',
        readerWithRefetchQueries: wrapResolvedValue({
          kind: 'ReaderWithRefetchQueries',
          readerArtifact: entrypoint.readerWithRefetchQueries.readerArtifact,
          nestedRefetchQueries:
            entrypoint.readerWithRefetchQueries.nestedRefetchQueries,
        }),
        root: { __link: ROOT_ID, __typename: entrypoint.concreteType },
        variables,
        networkRequest: networkRequest,
      },
      disposeNetworkRequest,
    ];
    return itemCleanupPair;
  };
  return getOrCreateItemInSuspenseCache(environment, cacheKey, factory);
}

type NetworkResponseScalarValue = string | number | boolean;
type NetworkResponseValue =
  | NetworkResponseScalarValue
  | null
  | NetworkResponseObject
  | (NetworkResponseObject | null)[]
  | (NetworkResponseScalarValue | null)[];

export type NetworkResponseObject = {
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the network response.
  [index: string]: undefined | NetworkResponseValue;
  id?: DataId;
  __typename?: TypeName;
};

export function normalizeData(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAstNodes,
  networkResponse: NetworkResponseObject,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  root: Link,
): EncounteredIds {
  const encounteredIds: EncounteredIds = new Map();

  logMessage(environment, {
    kind: 'AboutToNormalize',
    normalizationAst,
    networkResponse,
    variables,
  });

  const recordsById = (environment.store[root.__typename] ??= {});
  const newStoreRecord = (recordsById[root.__link] ??= {});

  normalizeDataIntoRecord(
    environment,
    normalizationAst,
    networkResponse,
    newStoreRecord,
    root,
    variables,
    nestedRefetchQueries,
    encounteredIds,
  );

  logMessage(environment, {
    kind: 'AfterNormalization',
    store: environment.store,
    encounteredIds,
  });

  callSubscriptions(environment, encounteredIds);
  return encounteredIds;
}

export function subscribeToAnyChange(
  environment: IsographEnvironment,
  callback: () => void,
): () => void {
  const subscription = {
    kind: 'AnyRecords',
    callback,
  } as const;
  environment.subscriptions.add(subscription);
  return () => environment.subscriptions.delete(subscription);
}

export function subscribeToAnyChangesToRecord(
  environment: IsographEnvironment,
  recordLink: Link,
  callback: () => void,
): () => void {
  const subscription = {
    kind: 'AnyChangesToRecord',
    recordLink,
    callback,
  } as const;
  environment.subscriptions.add(subscription);
  return () => environment.subscriptions.delete(subscription);
}

// TODO we should re-read and call callback if the value has changed
export function subscribe<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
>(
  environment: IsographEnvironment,
  encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  callback: (
    newEncounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  ) => void,
  readerAst: ReaderAst<TReadFromStore>,
): () => void {
  const fragmentSubscription: FragmentSubscription<TReadFromStore> = {
    kind: 'FragmentSubscription',
    callback,
    encounteredDataAndRecords,
    fragmentReference,
    readerAst,
  };
  environment.subscriptions.add(fragmentSubscription);
  return () => environment.subscriptions.delete(fragmentSubscription);
}

export function onNextChangeToRecord(
  environment: IsographEnvironment,
  recordLink: Link,
): Promise<void> {
  return new Promise((resolve) => {
    const unsubscribe = subscribeToAnyChangesToRecord(
      environment,
      recordLink,
      () => {
        unsubscribe();
        resolve();
      },
    );
  });
}

// Calls to readButDoNotEvaluate can suspend (i.e. throw a promise).
// Maybe in the future, they will be able to throw errors.
//
// That's probably okay to ignore. We don't, however, want to prevent
// updating other subscriptions if one subscription had missing data.
function withErrorHandling<T>(f: (t: T) => void): (t: T) => void {
  return (t) => {
    try {
      return f(t);
    } catch {}
  };
}

function callSubscriptions(
  environment: IsographEnvironment,
  recordsEncounteredWhenNormalizing: EncounteredIds,
) {
  environment.subscriptions.forEach(
    withErrorHandling((subscription) => {
      switch (subscription.kind) {
        case 'FragmentSubscription': {
          // TODO if there are multiple components subscribed to the same
          // fragment, we will call readButNotEvaluate multiple times. We
          // should fix that.
          if (
            hasOverlappingIds(
              recordsEncounteredWhenNormalizing,
              subscription.encounteredDataAndRecords.encounteredRecords,
            )
          ) {
            const newEncounteredDataAndRecords = readButDoNotEvaluate(
              environment,
              subscription.fragmentReference,
              // Is this wrong?
              // Reasons to think no:
              // - we are only updating the read-out value, and the network
              //   options only affect whether we throw.
              // - the component will re-render, and re-throw on its own, anyway.
              //
              // Reasons to think not:
              // - it seems more efficient to suspend here and not update state,
              //   if we expect that the component will just throw anyway
              // - consistency
              // - it's also weird, this is called from makeNetworkRequest, where
              //   we don't currently pass network request options
              {
                suspendIfInFlight: false,
                throwOnNetworkError: false,
              },
            );

            const mergedItem = mergeObjectsUsingReaderAst(
              subscription.readerAst,
              subscription.encounteredDataAndRecords.item,
              newEncounteredDataAndRecords.item,
            );

            logMessage(environment, {
              kind: 'DeepEqualityCheck',
              fragmentReference: subscription.fragmentReference,
              old: subscription.encounteredDataAndRecords.item,
              new: newEncounteredDataAndRecords.item,
              deeplyEqual:
                mergedItem === subscription.encounteredDataAndRecords.item,
            });

            if (mergedItem !== subscription.encounteredDataAndRecords.item) {
              subscription.callback(newEncounteredDataAndRecords);
            }
          }
          return;
        }
        case 'AnyRecords': {
          subscription.callback();
          return;
        }
        case 'AnyChangesToRecord': {
          if (
            recordsEncounteredWhenNormalizing
              .get(subscription.recordLink.__typename)
              ?.has(subscription.recordLink.__link)
          ) {
            subscription.callback();
          }
          return;
        }
        default: {
          // Ensure we have covered all variants
          const _: never = subscription;
          _;
          throw new Error('Unexpected case');
        }
      }
    }),
  );
}

function hasOverlappingIds(
  ids1: EncounteredIds,
  ids2: EncounteredIds,
): boolean {
  for (const [typeName, set1] of ids1.entries()) {
    const set2 = ids2.get(typeName);
    if (set2 === undefined) {
      continue;
    }

    if (isNotDisjointFrom(set1, set2)) {
      return true;
    }
  }
  return false;
}

// TODO use a polyfill library
function isNotDisjointFrom<T>(set1: Set<T>, set2: Set<T>): boolean {
  for (const id of set1) {
    if (set2.has(id)) {
      return true;
    }
  }
  return false;
}

export type EncounteredIds = Map<TypeName, Set<DataId>>;
/**
 * Mutate targetParentRecord according to the normalizationAst and networkResponseParentRecord.
 */
function normalizeDataIntoRecord(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAstNodes,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordLink: Link,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: EncounteredIds,
): RecordHasBeenUpdated {
  let recordHasBeenUpdated = false;
  for (const normalizationNode of normalizationAst) {
    switch (normalizationNode.kind) {
      case 'Scalar': {
        const scalarFieldResultedInChange = normalizeScalarField(
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          variables,
        );
        recordHasBeenUpdated =
          recordHasBeenUpdated || scalarFieldResultedInChange;
        break;
      }
      case 'Linked': {
        const linkedFieldResultedInChange = normalizeLinkedField(
          environment,
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordLink,
          variables,
          nestedRefetchQueries,
          mutableEncounteredIds,
        );
        recordHasBeenUpdated =
          recordHasBeenUpdated || linkedFieldResultedInChange;
        break;
      }
      case 'InlineFragment': {
        const inlineFragmentResultedInChange = normalizeInlineFragment(
          environment,
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordLink,
          variables,
          nestedRefetchQueries,
          mutableEncounteredIds,
        );
        recordHasBeenUpdated =
          recordHasBeenUpdated || inlineFragmentResultedInChange;
        break;
      }
      default: {
        // Ensure we have covered all variants
        let _: never = normalizationNode;
        _;
        throw new Error('Unexpected normalization node kind');
      }
    }
  }
  if (recordHasBeenUpdated) {
    let encounteredRecordsIds = insertIfNotExists(
      mutableEncounteredIds,
      targetParentRecordLink.__typename,
    );

    encounteredRecordsIds.add(targetParentRecordLink.__link);
  }
  return recordHasBeenUpdated;
}

export function insertIfNotExists<K, V>(map: Map<K, Set<V>>, key: K) {
  let result = map.get(key);
  if (result === undefined) {
    result = new Set();
    map.set(key, result);
  }
  return result;
}

type RecordHasBeenUpdated = boolean;
function normalizeScalarField(
  astNode: NormalizationScalarField,
  networkResponseParentRecord: NetworkResponseObject,
  targetStoreRecord: StoreRecord,
  variables: Variables,
): RecordHasBeenUpdated {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);

  if (
    networkResponseData == null ||
    isScalarOrEmptyArray(networkResponseData)
  ) {
    const existingValue = targetStoreRecord[parentRecordKey];
    targetStoreRecord[parentRecordKey] = networkResponseData;
    return existingValue !== networkResponseData;
  } else {
    throw new Error('Unexpected object array when normalizing scalar');
  }
}

/**
 * Mutate targetParentRecord with a given linked field ast node.
 */
function normalizeLinkedField(
  environment: IsographEnvironment,
  astNode: NormalizationLinkedField,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordLink: Link,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: EncounteredIds,
): RecordHasBeenUpdated {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);
  const existingValue = targetParentRecord[parentRecordKey];

  if (networkResponseData == null) {
    targetParentRecord[parentRecordKey] = null;
    return existingValue !== null;
  }

  if (
    isScalarOrEmptyArray(networkResponseData) &&
    !isNullOrEmptyArray(networkResponseData)
  ) {
    throw new Error(
      'Unexpected scalar network response when normalizing a linked field',
    );
  }

  if (Array.isArray(networkResponseData)) {
    // TODO check astNode.plural or the like
    const dataIds: (Link | null)[] = [];
    for (let i = 0; i < networkResponseData.length; i++) {
      const networkResponseObject = networkResponseData[i];
      if (networkResponseObject == null) {
        dataIds.push(null);
        continue;
      }
      const newStoreRecordId = normalizeNetworkResponseObject(
        environment,
        astNode,
        networkResponseObject,
        targetParentRecordLink,
        variables,
        i,
        nestedRefetchQueries,
        mutableEncounteredIds,
      );

      const __typename =
        astNode.concreteType ?? networkResponseObject[TYPENAME_FIELD_NAME];
      if (__typename == null) {
        throw new Error(
          'Unexpected missing __typename in network response when normalizing a linked field. ' +
            'This is indicative of a bug in Isograph.',
        );
      }
      dataIds.push({
        __link: newStoreRecordId,
        __typename,
      });
    }
    targetParentRecord[parentRecordKey] = dataIds;
    return !dataIdsAreTheSame(existingValue, dataIds);
  } else {
    const newStoreRecordId = normalizeNetworkResponseObject(
      environment,
      astNode,
      networkResponseData,
      targetParentRecordLink,
      variables,
      null,
      nestedRefetchQueries,
      mutableEncounteredIds,
    );

    let __typename =
      astNode.concreteType ?? networkResponseData[TYPENAME_FIELD_NAME];

    if (__typename == null) {
      throw new Error(
        'Unexpected missing __typename in network response when normalizing a linked field. ' +
          'This is indicative of a bug in Isograph.',
      );
    }

    targetParentRecord[parentRecordKey] = {
      __link: newStoreRecordId,
      __typename,
    };

    const link = getLink(existingValue);
    return link?.__link !== newStoreRecordId || link.__typename !== __typename;
  }
}

/**
 * Mutate targetParentRecord with a given linked field ast node.
 */
function normalizeInlineFragment(
  environment: IsographEnvironment,
  astNode: NormalizationInlineFragment,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordLink: Link,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: EncounteredIds,
): RecordHasBeenUpdated {
  const typeToRefineTo = astNode.type;
  if (networkResponseParentRecord[TYPENAME_FIELD_NAME] === typeToRefineTo) {
    const hasBeenModified = normalizeDataIntoRecord(
      environment,
      astNode.selections,
      networkResponseParentRecord,
      targetParentRecord,
      targetParentRecordLink,
      variables,
      nestedRefetchQueries,
      mutableEncounteredIds,
    );
    return hasBeenModified;
  }
  return false;
}

function dataIdsAreTheSame(
  existingValue: DataTypeValue,
  newDataIds: (Link | null)[],
): boolean {
  if (Array.isArray(existingValue)) {
    if (newDataIds.length !== existingValue.length) {
      return false;
    }
    for (let i = 0; i < newDataIds.length; i++) {
      const maybeLink = getLink(existingValue[i]);
      if (
        newDataIds[i]?.__link !== maybeLink?.__link ||
        newDataIds[i]?.__typename !== maybeLink?.__typename
      ) {
        return false;
      }
    }
    return true;
  } else {
    return false;
  }
}

function normalizeNetworkResponseObject(
  environment: IsographEnvironment,
  astNode: NormalizationLinkedField,
  networkResponseData: NetworkResponseObject,
  targetParentRecordLink: Link,
  variables: Variables,
  index: number | null,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: EncounteredIds,
): DataId /* The id of the modified or newly created item */ {
  const newStoreRecordId = getDataIdOfNetworkResponse(
    targetParentRecordLink,
    networkResponseData,
    astNode,
    variables,
    index,
  );
  const __typename =
    astNode.concreteType ?? networkResponseData[TYPENAME_FIELD_NAME];

  if (__typename == null) {
    throw new Error(
      'Unexpected missing __typename in network response object. ' +
        'This is indicative of a bug in Isograph.',
    );
  }

  const recordsById = (environment.store[__typename] ??= {});
  const newStoreRecord = (recordsById[newStoreRecordId] ??= {});

  normalizeDataIntoRecord(
    environment,
    astNode.selections,
    networkResponseData,
    newStoreRecord,
    { __link: newStoreRecordId, __typename: __typename },
    variables,
    nestedRefetchQueries,
    mutableEncounteredIds,
  );

  return newStoreRecordId;
}

function isScalarOrEmptyArray(
  data: NonNullable<NetworkResponseValue>,
): data is NetworkResponseScalarValue | (NetworkResponseScalarValue | null)[] {
  // N.B. empty arrays count as empty arrays of scalar fields.
  if (Array.isArray(data)) {
    // This is maybe fixed in a new version of Typescript??
    return (data as any).every((x: any) => isScalarOrEmptyArray(x));
  }
  const isScalarValue =
    data === null ||
    typeof data === 'string' ||
    typeof data === 'number' ||
    typeof data === 'boolean';
  return isScalarValue;
}

function isNullOrEmptyArray(data: unknown): data is never[] | null[] | null {
  if (Array.isArray(data)) {
    if (data.length === 0) {
      return true;
    }
    return data.every((x) => isNullOrEmptyArray(x));
  }

  return data === null;
}

export function getParentRecordKey(
  astNode:
    | NormalizationLinkedField
    | NormalizationScalarField
    | ReaderLinkedField
    | ReaderScalarField,
  variables: Variables,
): string {
  let parentRecordKey = astNode.fieldName;
  const fieldParameters = astNode.arguments;
  if (fieldParameters != null) {
    for (const fieldParameter of fieldParameters) {
      parentRecordKey += getStoreKeyChunkForArgument(fieldParameter, variables);
    }
  }

  return parentRecordKey;
}

function getStoreKeyChunkForArgumentValue(
  argumentValue: ArgumentValue,
  variables: Variables,
) {
  switch (argumentValue.kind) {
    case 'Literal': {
      return argumentValue.value;
    }
    case 'Variable': {
      return variables[argumentValue.name] ?? 'null';
    }
    case 'String': {
      return argumentValue.value;
    }
    case 'Enum': {
      return argumentValue.value;
    }
    default: {
      // TODO configure eslint to allow unused vars starting with _
      // Ensure we have covered all variants
      const _: never = argumentValue;
      _;
      throw new Error('Unexpected case');
    }
  }
}

function getStoreKeyChunkForArgument(argument: Argument, variables: Variables) {
  const chunk = getStoreKeyChunkForArgumentValue(argument[1], variables);
  return `${FIRST_SPLIT_KEY}${argument[0]}${SECOND_SPLIT_KEY}${chunk}`;
}

function getNetworkResponseKey(
  astNode: NormalizationLinkedField | NormalizationScalarField,
): string {
  let networkResponseKey = astNode.fieldName;
  const fieldParameters = astNode.arguments;
  if (fieldParameters != null) {
    for (const fieldParameter of fieldParameters) {
      const [argumentName, argumentValue] = fieldParameter;
      let argumentValueChunk;
      switch (argumentValue.kind) {
        case 'Literal': {
          argumentValueChunk = 'l_' + argumentValue.value;
          break;
        }
        case 'Variable': {
          argumentValueChunk = 'v_' + argumentValue.name;
          break;
        }
        case 'String': {
          argumentValueChunk = 's_' + argumentValue.value;
          break;
        }
        case 'Enum': {
          argumentValueChunk = 'e_' + argumentValue.value;
          break;
        }
        default: {
          // Ensure we have covered all variants
          let _: never = argumentValue;
          _;
          throw new Error('Unexpected case');
        }
      }
      networkResponseKey += `${FIRST_SPLIT_KEY}${argumentName}${SECOND_SPLIT_KEY}${argumentValueChunk}`;
    }
  }
  return networkResponseKey;
}

// an alias might be pullRequests____first___first____after___cursor
export const FIRST_SPLIT_KEY = '____';
export const SECOND_SPLIT_KEY = '___';

// Returns a key to look up an item in the store
function getDataIdOfNetworkResponse(
  parentRecordLink: Link,
  dataToNormalize: NetworkResponseObject,
  astNode: NormalizationLinkedField,
  variables: Variables,
  index: number | null,
): DataId {
  // If we are dealing with nested Query, use __ROOT as id
  // TODO do not hard code this value here
  if (astNode.concreteType === 'Query') {
    return ROOT_ID;
  }

  // Check whether the dataToNormalize has an id field. If so, that is the key.
  // If not, we construct an id from the parentRecordId and the field parameters.

  const dataId = dataToNormalize.id;
  if (dataId != null) {
    return dataId;
  }

  let storeKey = `${parentRecordLink.__typename}:${parentRecordLink.__link}.${astNode.fieldName}`;
  if (index != null) {
    storeKey += `.${index}`;
  }

  const fieldParameters = astNode.arguments;
  if (fieldParameters == null) {
    return storeKey;
  }

  for (const fieldParameter of fieldParameters) {
    storeKey += getStoreKeyChunkForArgument(fieldParameter, variables);
  }
  return storeKey;
}
