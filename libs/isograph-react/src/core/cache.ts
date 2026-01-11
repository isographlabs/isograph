import { type Factory, ParentCache } from '@isograph/react-disposable-state';
import type { Brand } from './brand';
import type {
  NormalizationAstNodes,
  NormalizationInlineFragment,
  NormalizationLinkedField,
  NormalizationScalarField,
} from './entrypoint';
import type {
  FragmentReference,
  UnknownTReadFromStore,
  Variables,
  VariableValue,
} from './FragmentReference';
import {
  type DataId,
  type DataTypeValueLinked,
  getLink,
  type IsographEnvironment,
  ROOT_ID,
  type StoreLink,
  type StoreRecord,
  type TypeName,
} from './IsographEnvironment';
import { logMessage } from './logging';
import {
  getMutableStoreRecordProxy,
  type StoreLayerWithData,
} from './optimisticProxy';
import type { ReaderLinkedField, ReaderScalarField } from './reader';
import { type Argument, type ArgumentValue, isArray, stableCopy } from './util';

export const TYPENAME_FIELD_NAME = '__typename';

export function getOrCreateItemInSuspenseCache<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  index: string,
  factory: Factory<FragmentReference<TReadFromStore, TClientFieldValue>>,
): ParentCache<FragmentReference<TReadFromStore, TClientFieldValue>> {
  if (environment.fragmentCache[index] == null) {
    environment.fragmentCache[index] = new ParentCache(factory);
  }

  return environment.fragmentCache[index];
}

export type NetworkResponsePlural<T> =
  | null
  | T
  | readonly T[]
  | readonly (null | T)[];
export type NetworkResponseScalarValue = string | number | boolean | unknown;

export type NetworkResponseValue =
  | NetworkResponsePlural<NetworkResponseScalarValue>
  | NetworkResponsePlural<NetworkResponseObject>;

export type NetworkResponseObject = {
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the network response.
  readonly [K in
    | ScalarNetworkResponseKey
    | LinkedNetworkResponseKey]: K extends ScalarNetworkResponseKey
    ? undefined | NetworkResponsePlural<NetworkResponseScalarValue>
    : undefined | NetworkResponsePlural<NetworkResponseObject>;
} & {
  readonly id?: DataId;
  readonly __typename?: TypeName;
};

export function normalizeData(
  environment: IsographEnvironment,
  storeLayer: StoreLayerWithData,
  normalizationAst: NormalizationAstNodes,
  networkResponse: NetworkResponseObject,
  variables: Variables,
  root: StoreLink,
  encounteredIds: EncounteredIds,
): EncounteredIds {
  logMessage(environment, () => ({
    kind: 'AboutToNormalize',
    normalizationAst,
    networkResponse,
    variables,
  }));

  const newStoreRecord = getMutableStoreRecordProxy(storeLayer, root);

  normalizeDataIntoRecord(
    environment,
    storeLayer,
    normalizationAst,
    networkResponse,
    newStoreRecord,
    root,
    variables,
    encounteredIds,
  );

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
  recordLink: StoreLink,
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

export function onNextChangeToRecord(
  environment: IsographEnvironment,
  recordLink: StoreLink,
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

export type EncounteredIds = Map<TypeName, Set<DataId>>;
/**
 * Mutate targetParentRecord according to the normalizationAst and networkResponseParentRecord.
 */
function normalizeDataIntoRecord(
  environment: IsographEnvironment,
  storeLayer: StoreLayerWithData,
  normalizationAst: NormalizationAstNodes,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordLink: StoreLink,
  variables: Variables,
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
          storeLayer,
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordLink,
          variables,
          mutableEncounteredIds,
        );
        recordHasBeenUpdated =
          recordHasBeenUpdated || linkedFieldResultedInChange;
        break;
      }
      case 'InlineFragment': {
        const inlineFragmentResultedInChange = normalizeInlineFragment(
          environment,
          storeLayer,
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordLink,
          variables,
          mutableEncounteredIds,
        );
        recordHasBeenUpdated =
          recordHasBeenUpdated || inlineFragmentResultedInChange;
        break;
      }
    }
  }
  if (recordHasBeenUpdated) {
    let encounteredRecordsIds = insertEmptySetIfMissing(
      mutableEncounteredIds,
      targetParentRecordLink.__typename,
    );

    encounteredRecordsIds.add(targetParentRecordLink.__link);
  }
  return recordHasBeenUpdated;
}

export function insertEmptySetIfMissing<K, V>(map: Map<K, Set<V>>, key: K) {
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
  const existingValue = targetStoreRecord[parentRecordKey];

  if (networkResponseData == null) {
    targetStoreRecord[parentRecordKey] = null;
    return existingValue === undefined || existingValue != null;
  }

  targetStoreRecord[parentRecordKey] = networkResponseData;
  return existingValue !== networkResponseData;
}

/**
 * Mutate targetParentRecord with a given linked field ast node.
 */
function normalizeLinkedField(
  environment: IsographEnvironment,
  storeLayer: StoreLayerWithData,
  astNode: NormalizationLinkedField,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordLink: StoreLink,
  variables: Variables,
  mutableEncounteredIds: EncounteredIds,
): RecordHasBeenUpdated {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);
  const existingValue = targetParentRecord[parentRecordKey];

  if (networkResponseData == null) {
    targetParentRecord[parentRecordKey] = null;
    return existingValue === undefined || existingValue != null;
  }

  if (isArray(networkResponseData)) {
    // TODO check astNode.plural or the like
    const dataIds: (StoreLink | null)[] = [];
    for (let i = 0; i < networkResponseData.length; i++) {
      const networkResponseObject = networkResponseData[i];
      if (networkResponseObject == null) {
        dataIds.push(null);
        continue;
      }
      const newStoreRecordId = normalizeNetworkResponseObject(
        environment,
        storeLayer,
        astNode,
        networkResponseObject,
        targetParentRecordLink,
        variables,
        i,
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
      storeLayer,
      astNode,
      networkResponseData,
      targetParentRecordLink,
      variables,
      null,
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
  storeLayer: StoreLayerWithData,
  astNode: NormalizationInlineFragment,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordLink: StoreLink,
  variables: Variables,
  mutableEncounteredIds: EncounteredIds,
): RecordHasBeenUpdated {
  const typeToRefineTo = astNode.type;
  if (networkResponseParentRecord[TYPENAME_FIELD_NAME] === typeToRefineTo) {
    const hasBeenModified = normalizeDataIntoRecord(
      environment,
      storeLayer,
      astNode.selections,
      networkResponseParentRecord,
      targetParentRecord,
      targetParentRecordLink,
      variables,
      mutableEncounteredIds,
    );
    return hasBeenModified;
  }
  return false;
}

function dataIdsAreTheSame(
  existingValue: DataTypeValueLinked,
  newDataIds: (StoreLink | null)[],
): boolean {
  if (isArray(existingValue)) {
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
  storeLayer: StoreLayerWithData,
  astNode: NormalizationLinkedField,
  networkResponseData: NetworkResponseObject,
  targetParentRecordLink: StoreLink,
  variables: Variables,
  index: number | null,
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

  const link = { __link: newStoreRecordId, __typename };
  const newStoreRecord = getMutableStoreRecordProxy(storeLayer, link);

  normalizeDataIntoRecord(
    environment,
    storeLayer,
    astNode.selections,
    networkResponseData,
    newStoreRecord,
    link,
    variables,
    mutableEncounteredIds,
  );

  return newStoreRecordId;
}

declare const LinkedParentRecordKeyBrand: unique symbol;
export type LinkedParentRecordKey = string & {
  brand?: Brand<undefined, typeof LinkedParentRecordKeyBrand>;
};

declare const ScalarParentRecordKeyBrand: unique symbol;
export type ScalarParentRecordKey = string & {
  brand?: Brand<undefined, typeof ScalarParentRecordKeyBrand>;
};

export function getParentRecordKey(
  astNode: NormalizationLinkedField | ReaderLinkedField,
  variables: Variables,
): LinkedParentRecordKey;
export function getParentRecordKey(
  astNode: NormalizationScalarField | ReaderScalarField,
  variables: Variables,
): ScalarParentRecordKey;
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
): VariableValue {
  switch (argumentValue.kind) {
    case 'Object': {
      return Object.fromEntries(
        argumentValue.value.map(([argumentName, argumentValue]) => {
          return [
            argumentName,
            //  substitute variables
            getStoreKeyChunkForArgumentValue(argumentValue, variables),
          ];
        }),
      );
    }
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
  }
}

function getStoreKeyChunkForArgument(argument: Argument, variables: Variables) {
  const [argumentName, argumentValue] = argument;
  let chunk = getStoreKeyChunkForArgumentValue(argumentValue, variables);

  if (typeof chunk === 'object') {
    chunk = JSON.stringify(stableCopy(chunk));
  }

  return `${FIRST_SPLIT_KEY}${argumentName}${SECOND_SPLIT_KEY}${chunk}`;
}

declare const LinkedNetworkResponseKeyBrand: unique symbol;
export type LinkedNetworkResponseKey = string & {
  brand?: Brand<undefined, typeof LinkedNetworkResponseKeyBrand>;
};

declare const ScalarNetworkResponseKeyBrand: unique symbol;
export type ScalarNetworkResponseKey = string & {
  brand?: Brand<undefined, typeof ScalarNetworkResponseKeyBrand>;
};

function getNetworkResponseKey(
  astNode: NormalizationLinkedField,
): LinkedNetworkResponseKey;
function getNetworkResponseKey(
  astNode: NormalizationScalarField,
): ScalarNetworkResponseKey;
function getNetworkResponseKey(
  astNode: NormalizationLinkedField | NormalizationScalarField,
): string {
  let networkResponseKey = astNode.fieldName;
  const fieldParameters = astNode.arguments;

  if (fieldParameters != null) {
    for (const [argumentName, argumentValue] of fieldParameters) {
      let argumentValueChunk = getArgumentValueChunk(argumentValue);
      networkResponseKey += `${FIRST_SPLIT_KEY}${argumentName}${SECOND_SPLIT_KEY}${argumentValueChunk}`;
    }
  }

  return networkResponseKey;
}

function getArgumentValueChunk(argumentValue: ArgumentValue): string {
  switch (argumentValue.kind) {
    case 'Object': {
      return (
        'o_' +
        argumentValue.value
          .map(([argumentName, argumentValue]) => {
            return (
              argumentName +
              THIRD_SPLIT_KEY +
              getArgumentValueChunk(argumentValue)
            );
          })
          .join('_') +
        '_c'
      );
    }
    case 'Literal': {
      return 'l_' + argumentValue.value;
    }
    case 'Variable': {
      return 'v_' + argumentValue.name;
    }
    case 'String': {
      // replace all non-word characters (alphanumeric & underscore) with underscores
      return 's_' + argumentValue.value.replaceAll(/\W/g, '_');
    }
    case 'Enum': {
      return 'e_' + argumentValue.value;
    }
  }
}

// an alias might be pullRequests____first___first____after___cursor
export const FIRST_SPLIT_KEY = '____';
export const SECOND_SPLIT_KEY = '___';
export const THIRD_SPLIT_KEY = '__';

// Returns a key to look up an item in the store
function getDataIdOfNetworkResponse(
  parentRecordLink: StoreLink,
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
