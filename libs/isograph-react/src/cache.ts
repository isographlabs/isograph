import {
  Factory,
  ItemCleanupPair,
  ParentCache,
} from '@isograph/react-disposable-state';
import { PromiseWrapper, wrapPromise } from './PromiseWrapper';
import {
  DataId,
  ROOT_ID,
  StoreRecord,
  Link,
  type IsographEnvironment,
} from './IsographEnvironment';
import {
  RetainedQuery,
  garbageCollectEnvironment,
  retainQuery,
  unretainQuery,
} from './garbageCollection';
import {
  IsographEntrypoint,
  NormalizationAst,
  NormalizationLinkedField,
  NormalizationScalarField,
  RefetchQueryArtifactWrapper,
} from './entrypoint';
import { ReaderLinkedField, ReaderScalarField } from './reader';
import { Argument, ArgumentValue } from './util';

declare global {
  interface Window {
    __LOG: boolean;
  }
}

function getOrCreateCache<T>(
  environment: IsographEnvironment,
  index: string,
  factory: Factory<T>,
): ParentCache<T> {
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('getting cache for', {
      index,
      cache: Object.keys(environment.suspenseCache),
      found: !!environment.suspenseCache[index],
    });
  }
  if (environment.suspenseCache[index] == null) {
    environment.suspenseCache[index] = new ParentCache(factory);
  }

  return environment.suspenseCache[index];
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

type IsoResolver = IsographEntrypoint<any, any>;

export function getOrCreateCacheForArtifact<
  TReadFromStore extends Object,
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: object,
): ParentCache<PromiseWrapper<TClientFieldValue>> {
  const cacheKey = artifact.queryText + JSON.stringify(stableCopy(variables));
  const factory: Factory<PromiseWrapper<TClientFieldValue>> = () =>
    makeNetworkRequest<TClientFieldValue>(environment, artifact, variables);
  return getOrCreateCache<PromiseWrapper<TClientFieldValue>>(
    environment,
    cacheKey,
    factory,
  );
}

type NetworkRequestStatus =
  | {
      kind: 'UndisposedIncomplete';
    }
  | {
      kind: 'Disposed';
    }
  | {
      kind: 'UndisposedComplete';
      retainedQuery: RetainedQuery;
    };

export function makeNetworkRequest<T>(
  environment: IsographEnvironment,
  artifact: IsoResolver,
  variables: object,
): ItemCleanupPair<PromiseWrapper<T>> {
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('make network request', artifact, variables);
  }
  let status: NetworkRequestStatus = {
    kind: 'UndisposedIncomplete',
  };
  // This should be an observable, not a promise
  const promise = environment
    .networkFunction(artifact.queryText, variables)
    .then((networkResponse) => {
      if (typeof window !== 'undefined' && window.__LOG) {
        console.log('network response', artifact, artifact);
      }

      if (status.kind === 'UndisposedIncomplete') {
        normalizeData(
          environment,
          artifact.normalizationAst,
          networkResponse.data,
          variables,
          artifact.nestedRefetchQueries,
        );
        const retainedQuery = {
          normalizationAst: artifact.normalizationAst,
          variables,
        };
        status = {
          kind: 'UndisposedComplete',
          retainedQuery,
        };
        retainQuery(environment, retainedQuery);
      }
      // TODO return null
      return networkResponse;
    });

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<T>> = [
    wrapper,
    () => {
      if (status.kind === 'UndisposedComplete') {
        const didUnretainSomeQuery = unretainQuery(
          environment,
          status.retainedQuery,
        );
        if (didUnretainSomeQuery) {
          garbageCollectEnvironment(environment);
        }
      }
      status = {
        kind: 'Disposed',
      };
    },
  ];
  return response;
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

function normalizeData(
  environment: IsographEnvironment,
  normalizationAst: NormalizationAst,
  networkResponse: NetworkResponseObject,
  variables: Object,
  nestedRefetchQueries: RefetchQueryArtifactWrapper[],
): Set<DataId> {
  const encounteredIds = new Set<DataId>();

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
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('after normalization', { store: environment.store });
  }
  callSubscriptions(environment);
  return encounteredIds;
}

export function subscribe(
  environment: IsographEnvironment,
  callback: () => void,
): () => void {
  environment.subscriptions.add(callback);
  return () => environment.subscriptions.delete(callback);
}

export function onNextChange(environment: IsographEnvironment): Promise<void> {
  return new Promise((resolve) => {
    const unsubscribe = subscribe(environment, () => {
      unsubscribe();
      resolve();
    });
  });
}

function callSubscriptions(environment: IsographEnvironment) {
  environment.subscriptions.forEach((callback) => callback());
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
  variables: { [index: string]: string },
  nestedRefetchQueries: RefetchQueryArtifactWrapper[],
  mutableEncounteredIds: Set<DataId>,
) {
  mutableEncounteredIds.add(targetParentRecordId);
  for (const normalizationNode of normalizationAst) {
    switch (normalizationNode.kind) {
      case 'Scalar': {
        normalizeScalarField(
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          variables,
        );
        break;
      }
      case 'Linked': {
        normalizeLinkedField(
          environment,
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordId,
          variables,
          nestedRefetchQueries,
          mutableEncounteredIds,
        );
        break;
      }
    }
  }
}

function normalizeScalarField(
  astNode: NormalizationScalarField,
  networkResponseParentRecord: NetworkResponseObject,
  targetStoreRecord: StoreRecord,
  variables: { [index: string]: string },
) {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);

  if (
    networkResponseData == null ||
    isScalarOrEmptyArray(networkResponseData)
  ) {
    targetStoreRecord[parentRecordKey] = networkResponseData;
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
  variables: { [index: string]: string },
  nestedRefetchQueries: RefetchQueryArtifactWrapper[],
  mutableEncounteredIds: Set<DataId>,
) {
  const networkResponseKey = getNetworkResponseKey(astNode);
  const networkResponseData = networkResponseParentRecord[networkResponseKey];
  const parentRecordKey = getParentRecordKey(astNode, variables);

  if (networkResponseData == null) {
    targetParentRecord[parentRecordKey] = null;
    return;
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
  }
}

function normalizeNetworkResponseObject(
  environment: IsographEnvironment,
  astNode: NormalizationLinkedField,
  networkResponseData: NetworkResponseObject,
  targetParentRecordId: string,
  variables: { [index: string]: string },
  index: number | null,
  nestedRefetchQueries: RefetchQueryArtifactWrapper[],
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
  variables: { [index: string]: string },
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
  variables: { [index: string]: string },
) {
  switch (argumentValue.kind) {
    case 'Literal': {
      return argumentValue.value;
    }
    case 'Variable': {
      return variables[argumentValue.name];
    }
    default: {
      // TODO configure eslint to allow unused vars starting with _
      // @ts-expect-error
      const _: never = argumentValue;
      throw new Error('Unexpected case');
    }
  }
}

function getStoreKeyChunkForArgument(
  argument: Argument,
  variables: { [index: string]: string },
) {
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
        default: {
          // @ts-expect-error
          let _: never = argumentValue;
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
  variables: { [index: string]: string },
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
