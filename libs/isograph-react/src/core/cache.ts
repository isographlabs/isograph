import {
  Factory,
  ItemCleanupPair,
  ParentCache,
} from '@isograph/react-disposable-state';
import {
  DataId,
  ROOT_ID,
  StoreRecord,
  Link,
  type IsographEnvironment,
  DataTypeValue,
  getLink,
  FragmentSubscription,
} from './IsographEnvironment';
import {
  IsographEntrypoint,
  NormalizationAst,
  NormalizationInlineFragment,
  NormalizationLinkedField,
  NormalizationScalarField,
  RefetchQueryNormalizationArtifactWrapper,
} from '../core/entrypoint';
import { ReaderLinkedField, ReaderScalarField, type ReaderAst } from './reader';
import { Argument, ArgumentValue } from './util';
import { WithEncounteredRecords, readButDoNotEvaluate } from './read';
import { FragmentReference, Variables } from './FragmentReference';
import { mergeObjectsUsingReaderAst } from './areEqualWithDeepComparison';
import { makeNetworkRequest } from './makeNetworkRequest';
import { wrapResolvedValue } from './PromiseWrapper';

const TYPENAME_FIELD_NAME = '__typename';

export function getOrCreateItemInSuspenseCache<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  index: string,
  factory: Factory<FragmentReference<TReadFromStore, TClientFieldValue>>,
): ParentCache<FragmentReference<TReadFromStore, TClientFieldValue>> {
  // @ts-expect-error
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('getting cache for', {
      index,
      cache: Object.keys(environment.fragmentCache),
      found: !!environment.fragmentCache[index],
    });
  }
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
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: Variables,
): ParentCache<FragmentReference<TReadFromStore, TClientFieldValue>> {
  const cacheKey = entrypoint.queryText + JSON.stringify(stableCopy(variables));
  const factory = () => {
    const [networkRequest, disposeNetworkRequest] = makeNetworkRequest(
      environment,
      entrypoint,
      variables,
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
        root: ROOT_ID,
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
  | NetworkResponseObject[]
  | NetworkResponseScalarValue[];
type NetworkResponseObject = {
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the network response.
  [index: string]: undefined | NetworkResponseValue;
  id?: DataId;
};

export function normalizeData(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAst,
  networkResponse: NetworkResponseObject,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
): Set<DataId> {
  const encounteredIds = new Set<DataId>();

  // @ts-expect-error
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log(
      'about to normalize',
      normalizationAst,
      networkResponse,
      variables,
    );
  }
  normalizeDataIntoRecord(
    environment,
    normalizationAst,
    networkResponse,
    environment.store.__ROOT,
    ROOT_ID,
    variables as any,
    nestedRefetchQueries,
    encounteredIds,
  );
  // @ts-expect-error
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('after normalization', {
      store: environment.store,
      encounteredIds,
      environment,
    });
  }
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

// TODO we should re-read and call callback if the value has changed
export function subscribe<
  TReadFromStore extends { parameters: object; data: object },
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
  // @ts-expect-error
  environment.subscriptions.add(fragmentSubscription);
  // @ts-expect-error
  return () => environment.subscriptions.delete(fragmentSubscription);
}

export function onNextChange(environment: IsographEnvironment): Promise<void> {
  return new Promise((resolve) => {
    const unsubscribe = subscribeToAnyChange(environment, () => {
      unsubscribe();
      resolve();
    });
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
  recordsEncounteredWhenNormalizing: Set<DataId>,
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
            if (mergedItem !== subscription.encounteredDataAndRecords.item) {
              // @ts-expect-error
              if (typeof window !== 'undefined' && window.__LOG) {
                console.log('Deep equality - No', {
                  fragmentReference: subscription.fragmentReference,
                  old: subscription.encounteredDataAndRecords.item,
                  new: newEncounteredDataAndRecords.item,
                });
              }
              subscription.callback(newEncounteredDataAndRecords);
            } else {
              // @ts-expect-error
              if (typeof window !== 'undefined' && window.__LOG) {
                console.log('Deep equality - Yes', {
                  fragmentReference: subscription.fragmentReference,
                  old: subscription.encounteredDataAndRecords.item,
                });
              }
            }
          }
          return;
        }
        case 'AnyRecords': {
          return subscription.callback();
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

function hasOverlappingIds(set1: Set<DataId>, set2: Set<DataId>): boolean {
  for (const id of set1) {
    if (set2.has(id)) {
      return true;
    }
  }
  return false;
}

/**
 * Mutate targetParentRecord according to the normalizationAst and networkResponseParentRecord.
 */
function normalizeDataIntoRecord(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAst,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordId: DataId,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: Set<DataId>,
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
          targetParentRecordId,
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
          targetParentRecordId,
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
    mutableEncounteredIds.add(targetParentRecordId);
  }
  return recordHasBeenUpdated;
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
  targetParentRecordId: DataId,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: Set<DataId>,
): RecordHasBeenUpdated {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);
  const existingValue = targetParentRecord[parentRecordKey];

  if (networkResponseData == null) {
    targetParentRecord[parentRecordKey] = null;
    return existingValue !== null;
  }

  if (isScalarButNotEmptyArray(networkResponseData)) {
    throw new Error(
      'Unexpected scalar network response when normalizing a linked field',
    );
  }

  if (Array.isArray(networkResponseData)) {
    // TODO check astNode.plural or the like
    const dataIds: Link[] = [];
    for (let i = 0; i < networkResponseData.length; i++) {
      const networkResponseObject = networkResponseData[i];
      const newStoreRecordId = normalizeNetworkResponseObject(
        environment,
        astNode,
        networkResponseObject,
        targetParentRecordId,
        variables,
        i,
        nestedRefetchQueries,
        mutableEncounteredIds,
      );

      dataIds.push({ __link: newStoreRecordId });
    }
    targetParentRecord[parentRecordKey] = dataIds;
    return !dataIdsAreTheSame(existingValue, dataIds);
  } else {
    const newStoreRecordId = normalizeNetworkResponseObject(
      environment,
      astNode,
      networkResponseData,
      targetParentRecordId,
      variables,
      null,
      nestedRefetchQueries,
      mutableEncounteredIds,
    );

    targetParentRecord[parentRecordKey] = {
      __link: newStoreRecordId,
    };
    const link = getLink(existingValue);
    return link?.__link !== newStoreRecordId;
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
  targetParentRecordId: DataId,
  variables: Variables,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: Set<DataId>,
): RecordHasBeenUpdated {
  const typeToRefineTo = astNode.type;
  if (networkResponseParentRecord[TYPENAME_FIELD_NAME] === typeToRefineTo) {
    const hasBeenModified = normalizeDataIntoRecord(
      environment,
      astNode.selections,
      networkResponseParentRecord,
      targetParentRecord,
      targetParentRecordId,
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
  newDataIds: Link[],
): boolean {
  if (Array.isArray(existingValue)) {
    if (newDataIds.length !== existingValue.length) {
      return false;
    }
    for (let i = 0; i < newDataIds.length; i++) {
      const maybeLink = getLink(existingValue[i]);
      if (maybeLink !== null) {
        if (newDataIds[i].__link !== maybeLink.__link) {
          return false;
        }
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
  targetParentRecordId: string,
  variables: Variables,
  index: number | null,
  nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[],
  mutableEncounteredIds: Set<DataId>,
): DataId /* The id of the modified or newly created item */ {
  const newStoreRecordId = getDataIdOfNetworkResponse(
    targetParentRecordId,
    networkResponseData,
    astNode,
    variables,
    index,
  );

  const newStoreRecord = environment.store[newStoreRecordId] ?? {};
  environment.store[newStoreRecordId] = newStoreRecord;

  normalizeDataIntoRecord(
    environment,
    astNode.selections,
    networkResponseData,
    newStoreRecord,
    newStoreRecordId,
    variables,
    nestedRefetchQueries,
    mutableEncounteredIds,
  );

  return newStoreRecordId;
}

function isScalarOrEmptyArray(
  data: NonNullable<NetworkResponseValue>,
): data is NetworkResponseScalarValue | NetworkResponseScalarValue[] {
  // N.B. empty arrays count as empty arrays of scalar fields.
  if (Array.isArray(data)) {
    // This is maybe fixed in a new version of Typescript??
    return (data as any).every((x: any) => isScalarOrEmptyArray(x));
  }
  const isScalarValue =
    typeof data === 'string' ||
    typeof data === 'number' ||
    typeof data === 'boolean';
  return isScalarValue;
}

function isScalarButNotEmptyArray(
  data: NonNullable<NetworkResponseValue>,
): data is NetworkResponseScalarValue | NetworkResponseScalarValue[] {
  // N.B. empty arrays count as empty arrays of linked fields.
  if (Array.isArray(data)) {
    if (data.length === 0) {
      return false;
    }
    // This is maybe fixed in a new version of Typescript??
    return (data as any).every((x: any) => isScalarOrEmptyArray(x));
  }
  const isScalarValue =
    typeof data === 'string' ||
    typeof data === 'number' ||
    typeof data === 'boolean';
  return isScalarValue;
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
  parentRecordId: DataId,
  dataToNormalize: NetworkResponseObject,
  astNode: NormalizationLinkedField | NormalizationScalarField,
  variables: Variables,
  index: number | null,
): DataId {
  // Check whether the dataToNormalize has an id field. If so, that is the key.
  // If not, we construct an id from the parentRecordId and the field parameters.

  const dataId = dataToNormalize.id;
  if (dataId != null) {
    return dataId;
  }

  let storeKey = `${parentRecordId}.${astNode.fieldName}`;
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
