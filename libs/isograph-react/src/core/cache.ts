import { type Factory, ParentCache } from '@isograph/react-disposable-state';
import type {
  NormalizationAstNodes,
  NormalizationInlineFragment,
  NormalizationLinkedField,
  NormalizationScalarField,
} from '../core/entrypoint';
import type { Brand } from './brand';
import type {
  FragmentReference,
  UnknownTReadFromStore,
  Variables,
  VariableValue,
} from './FragmentReference';
import {
  type DataId,
  type DataTypeValue,
  getLink,
  type IsographEnvironment,
  type PayloadError,
  type PayloadErrorPath,
  type PayloadErrors,
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

export type NetworkResponseScalarValue = string | number | boolean;
export type NetworkResponseValue =
  | NetworkResponseScalarValue
  | null
  | NetworkResponseObject
  | readonly (NetworkResponseObject | null)[]
  | readonly (NetworkResponseScalarValue | null)[];

export type NetworkResponseObject = {
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the network response.
  readonly [index: string]: undefined | NetworkResponseValue;
  readonly id?: DataId;
  readonly __typename?: TypeName;
};

declare const PayloadErrorPathJoinedBrand: unique symbol;
type PayloadErrorPathJoined = Brand<string, typeof PayloadErrorPathJoinedBrand>;

function joinPayloadErrorPath(
  path: PayloadErrorPath[] | undefined,
): PayloadErrorPathJoined {
  return (path?.join('.') ?? '') as PayloadErrorPathJoined;
}

export type ErrorsByPath = Partial<
  Record<PayloadErrorPathJoined, PayloadErrors>
>;

export function normalizeData(
  environment: IsographEnvironment,
  storeLayer: StoreLayerWithData,
  normalizationAst: NormalizationAstNodes,
  networkResponse: {
    data: NetworkResponseObject | undefined;
    errors: PayloadErrors | undefined;
  },
  variables: Variables,
  root: StoreLink,
  encounteredIds: EncounteredIds,
): EncounteredIds {
  logMessage(environment, () => ({
    kind: 'AboutToNormalize',
    normalizationAst,
    networkResponse: networkResponse.data,
    errors: networkResponse.errors,
    variables,
  }));

  const newStoreRecord = getMutableStoreRecordProxy(storeLayer, root);

  const errorsByPath: ErrorsByPath = groupBy<
    PayloadError,
    PayloadErrorPathJoined
  >(networkResponse.errors ?? [], (error) => joinPayloadErrorPath(error.path));

  const path: PayloadErrorPath[] = [];

  normalizeDataIntoRecord(
    environment,
    storeLayer,
    normalizationAst,
    networkResponse.data ?? {},
    newStoreRecord,
    root,
    variables,
    encounteredIds,
    errorsByPath,
    path,
  );

  return encounteredIds;
}

function groupBy<V, K extends string | number | symbol>(
  arr: readonly V[],
  keyFn: (v: V) => K,
) {
  const result: Partial<Record<K, [V, ...V[]]>> = {};
  for (const el of arr) {
    const key = keyFn(el);
    if (result[key] != null) {
      result[key].push(el);
    } else {
      result[key] = [el];
    }
  }
  return result;
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
  errorsByPath: ErrorsByPath,
  path: PayloadErrorPath[],
): RecordHasBeenUpdated {
  let recordHasBeenUpdated = false;
  for (const normalizationNode of normalizationAst) {
    switch (normalizationNode.kind) {
      case 'Scalar': {
        const networkResponseKey = getNetworkResponseKey(normalizationNode);
        path.push(networkResponseKey);
        const scalarFieldResultedInChange = normalizeScalarField(
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          variables,
          errorsByPath,
          path,
        );
        path.pop();
        recordHasBeenUpdated =
          recordHasBeenUpdated || scalarFieldResultedInChange;
        break;
      }
      case 'Linked': {
        const networkResponseKey = getNetworkResponseKey(normalizationNode);
        path.push(networkResponseKey);
        const linkedFieldResultedInChange = normalizeLinkedField(
          environment,
          storeLayer,
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordLink,
          variables,
          mutableEncounteredIds,
          errorsByPath,
          path,
        );
        path.pop();
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
          errorsByPath,
          path,
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

/**
 * If errors bubble up, the error path will be a full-path to the field
 */
function findErrors(errorsByPath: ErrorsByPath, path: PayloadErrorPath[]) {
  const joinedPath = joinPayloadErrorPath(path);
  let errors: PayloadErrors | undefined = undefined;
  for (const [errorPath, suberrors] of Object.entries(errorsByPath) as Iterable<
    [PayloadErrorPathJoined, PayloadErrors]
  >) {
    if (suberrors != null && errorPath.startsWith(joinedPath)) {
      if (errors == null) {
        errors = suberrors;
      } else {
        errors.push(...suberrors);
      }
    }
  }
  return errors;
}

type RecordHasBeenUpdated = boolean;
function normalizeScalarField(
  astNode: NormalizationScalarField,
  networkResponseParentRecord: NetworkResponseObject,
  targetStoreRecord: StoreRecord,
  variables: Variables,
  errorsByPath: ErrorsByPath,
  path: PayloadErrorPath[],
): RecordHasBeenUpdated {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);
  const existingValue = targetStoreRecord[parentRecordKey];

  if (networkResponseData == null) {
    const errors = findErrors(errorsByPath, path);

    if (errors != null) {
      targetStoreRecord[parentRecordKey] = {
        kind: 'Errors',
        errors,
      };
      return (
        existingValue?.kind !== 'Errors' ||
        JSON.stringify(stableCopy(existingValue.errors)) !==
          JSON.stringify(stableCopy(errors))
      );
    }
    targetStoreRecord[parentRecordKey] = {
      kind: 'Data',
      value: null,
      errors: undefined,
    };
    return (
      existingValue?.kind === 'Errors' ||
      existingValue?.value === undefined ||
      existingValue != null
    );
  }

  if (isScalarOrEmptyArray(networkResponseData)) {
    targetStoreRecord[parentRecordKey] = {
      kind: 'Data',
      value: networkResponseData,
      errors: targetStoreRecord[parentRecordKey]?.errors,
    };
    return (
      existingValue?.kind === 'Errors' ||
      existingValue?.value !== networkResponseData
    );
  } else {
    throw new Error('Unexpected object array when normalizing scalar');
  }
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
  errorsByPath: ErrorsByPath,
  path: PayloadErrorPath[],
): RecordHasBeenUpdated {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);
  const existingValue = targetParentRecord[parentRecordKey];

  if (networkResponseData == null) {
    const errors = findErrors(errorsByPath, path);

    if (errors != null) {
      targetParentRecord[parentRecordKey] = {
        kind: 'Errors',
        errors,
      };
      return (
        existingValue?.kind !== 'Errors' ||
        JSON.stringify(stableCopy(existingValue.errors)) !==
          JSON.stringify(stableCopy(errors))
      );
    }
    targetParentRecord[parentRecordKey] = {
      kind: 'Data',
      value: null,
      errors: undefined,
    };
    return (
      existingValue?.kind === 'Errors' ||
      existingValue?.value === undefined ||
      existingValue != null
    );
  }

  if (
    isScalarOrEmptyArray(networkResponseData) &&
    !isNullOrEmptyArray(networkResponseData)
  ) {
    throw new Error(
      'Unexpected scalar network response when normalizing a linked field',
    );
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
      path.push(i as PayloadErrorPath);
      const newStoreRecordId = normalizeNetworkResponseObject(
        environment,
        storeLayer,
        astNode,
        networkResponseObject,
        targetParentRecordLink,
        variables,
        i,
        mutableEncounteredIds,
        errorsByPath,
        path,
      );
      path.pop();

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
    targetParentRecord[parentRecordKey] = {
      kind: 'Data',
      value: dataIds,
      errors: targetParentRecord[parentRecordKey]?.errors,
    };
    return (
      existingValue?.kind === 'Errors' ||
      !dataIdsAreTheSame(existingValue?.value, dataIds)
    );
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
      errorsByPath,
      path,
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
      kind: 'Data',
      value: {
        __link: newStoreRecordId,
        __typename,
      },
      errors: targetParentRecord[parentRecordKey]?.errors,
    };

    const link =
      existingValue?.kind === 'Data' ? getLink(existingValue.value) : undefined;
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
  errorsByPath: ErrorsByPath,
  path: PayloadErrorPath[],
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
      errorsByPath,
      path,
    );
    return hasBeenModified;
  }
  return false;
}

function dataIdsAreTheSame(
  existingValue: DataTypeValue,
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
  errorsByPath: ErrorsByPath,
  path: PayloadErrorPath[],
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
    errorsByPath,
    path,
  );

  return newStoreRecordId;
}

function isScalarOrEmptyArray(
  data: NetworkResponseValue,
): data is
  | NetworkResponseScalarValue
  | readonly (NetworkResponseScalarValue | null)[] {
  // N.B. empty arrays count as empty arrays of scalar fields.
  if (isArray(data)) {
    return data.every((x) => isScalarOrEmptyArray(x));
  }
  const isScalarValue =
    data == null ||
    typeof data === 'string' ||
    typeof data === 'number' ||
    typeof data === 'boolean';
  return isScalarValue;
}

function isNullOrEmptyArray(
  data: unknown,
): data is readonly never[] | null[] | null {
  if (isArray(data)) {
    if (data.length === 0) {
      return true;
    }
    return data.every((x) => isNullOrEmptyArray(x));
  }

  return data == null;
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

export type NetworkResponseKey = string;

function getNetworkResponseKey(
  astNode: NormalizationLinkedField | NormalizationScalarField,
): NetworkResponseKey {
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
