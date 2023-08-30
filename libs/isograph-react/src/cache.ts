import {
  Factory,
  ItemCleanupPair,
  ParentCache,
} from "@isograph/react-disposable-state";
import { PromiseWrapper, wrapPromise } from "./PromiseWrapper";
import {
  IsographFetchableResolver,
  NormalizationAst,
  NormalizationLinkedField,
  NormalizationScalarField,
  ReaderLinkedField,
  ReaderScalarField,
} from "./index";

const cache: { [index: string]: ParentCache<any> } = {};

function getOrCreateCache<T>(
  index: string,
  factory: Factory<T>
): ParentCache<T> {
  console.log("getting cache for", {
    index,
    cache: Object.keys(cache),
    found: !!cache[index],
  });
  if (cache[index] == null) {
    cache[index] = new ParentCache(factory);
  }

  return cache[index];
}

/**
 * Creates a copy of the provided value, ensuring any nested objects have their
 * keys sorted such that equivalent values would have identical JSON.stringify
 * results.
 */
function stableCopy<T>(value: T): T {
  if (!value || typeof value !== "object") {
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

type IsoResolver = IsographFetchableResolver<any, any, any>;

export function getOrCreateCacheForArtifact<T>(
  artifact: IsoResolver,
  variables: object
): ParentCache<PromiseWrapper<T>> {
  const cacheKey = artifact.queryText + JSON.stringify(stableCopy(variables));
  const factory: Factory<PromiseWrapper<T>> = () =>
    makeNetworkRequest<T>(artifact, variables);
  return getOrCreateCache<PromiseWrapper<T>>(cacheKey, factory);
}

let network: ((queryText: string, variables: object) => Promise<any>) | null;

// This is a hack until we store this in context somehow
export function setNetwork(newNetwork: typeof network) {
  network = newNetwork;
}

export function makeNetworkRequest<T>(
  artifact: IsoResolver,
  variables: object
): ItemCleanupPair<PromiseWrapper<T>> {
  console.log("make network request", artifact, variables);
  if (network == null) {
    throw new Error("Network must be set before makeNetworkRequest is called");
  }

  const promise = network(artifact.queryText, variables).then(
    (networkResponse) => {
      console.log("network response", artifact);
      normalizeData(artifact.normalizationAst, networkResponse.data, variables);
      return networkResponse.data;
    }
  );

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<T>> = [
    wrapper,
    () => {
      // delete from cache
    },
  ];
  return response;
}

export type Link = {
  __link: DataId;
};
export type DataTypeValue =
  // N.B. undefined is here to support optional id's, but
  // undefined should not *actually* be present in the store.
  | undefined
  // Singular scalar fields:
  | number
  | boolean
  | string
  | null
  // Singular linked fields:
  | Link
  // Plural scalar and linked fields:
  | DataTypeValue[];

export type StoreRecord = {
  [index: DataId | string]: DataTypeValue;
  // TODO __typename?: T, which is restricted to being a concrete string
  // TODO this shouldn't always be named id
  id?: DataId;
};

export type DataId = string;

export const ROOT_ID: DataId & "__ROOT" = "__ROOT";
export const store: {
  [index: DataId]: StoreRecord | null;
  __ROOT: StoreRecord;
} = {
  __ROOT: {},
};

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
  normalizationAst: NormalizationAst,
  networkResponse: NetworkResponseObject,
  variables: Object
) {
  console.log(
    "about to normalize",
    normalizationAst,
    networkResponse,
    variables
  );
  normalizeDataIntoRecord(
    normalizationAst,
    networkResponse,
    store.__ROOT,
    ROOT_ID,
    variables as any
  );
  console.log("after normalization", { store });
  callSubscriptions();
}

export function subscribe(callback: () => void): () => void {
  subscriptions.add(callback);
  return () => subscriptions.delete(callback);
}

export function onNextChange(): Promise<void> {
  return new Promise((resolve) => {
    const unsubscribe = subscribe(() => {
      unsubscribe();
      resolve();
    });
  });
}

const subscriptions: Set<() => void> = new Set();

function callSubscriptions() {
  subscriptions.forEach((callback) => callback());
}

/**
 * Mutate targetParentRecord according to the normalizationAst and networkResponseParentRecord.
 */
function normalizeDataIntoRecord(
  normalizationAst: NormalizationAst,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordId: DataId,
  variables: { [index: string]: string }
) {
  for (const normalizationNode of normalizationAst) {
    switch (normalizationNode.kind) {
      case "Scalar": {
        normalizeScalarField(
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          variables
        );
        break;
      }
      case "Linked": {
        normalizeLinkedField(
          normalizationNode,
          networkResponseParentRecord,
          targetParentRecord,
          targetParentRecordId,
          variables
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
  variables: { [index: string]: string }
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
    throw new Error("Unexpected object array when normalizing scalar");
  }
}

/**
 * Mutate targetParentRecord with a given linked field ast node.
 */
function normalizeLinkedField(
  astNode: NormalizationLinkedField,
  networkResponseParentRecord: NetworkResponseObject,
  targetParentRecord: StoreRecord,
  targetParentRecordId: DataId,
  variables: { [index: string]: string }
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
      "Unexpected scalar network response when normalizing a linked field"
    );
  }

  if (Array.isArray(networkResponseData)) {
    // TODO check astNode.plural or the like
    const dataIds = [];
    for (let i = 0; i < networkResponseData.length; i++) {
      const networkResponseObject = networkResponseData[i];
      const newStoreRecordId = normalizeNetworkResponseObject(
        astNode,
        networkResponseObject,
        targetParentRecordId,
        variables,
        i
      );
      dataIds.push({ __link: newStoreRecordId });
    }
    targetParentRecord[parentRecordKey] = dataIds;
  } else {
    const newStoreRecordId = normalizeNetworkResponseObject(
      astNode,
      networkResponseData,
      targetParentRecordId,
      variables
    );
    targetParentRecord[parentRecordKey] = {
      __link: newStoreRecordId,
    };
  }
}

function normalizeNetworkResponseObject(
  astNode: NormalizationLinkedField,
  networkResponseData: NetworkResponseObject,
  targetParentRecordId: string,
  variables: { [index: string]: string },
  index?: number
): DataId /* The id of the modified or newly created item */ {
  const newStoreRecordId = getDataIdOfNetworkResponse(
    targetParentRecordId,
    networkResponseData,
    astNode,
    variables,
    index
  );

  const newStoreRecord = store[newStoreRecordId] ?? {};
  store[newStoreRecordId] = newStoreRecord;

  normalizeDataIntoRecord(
    astNode.selections,
    networkResponseData,
    newStoreRecord,
    newStoreRecordId,
    variables
  );

  return newStoreRecordId;
}

function isScalarOrEmptyArray(
  data: NonNullable<NetworkResponseValue>
): data is NetworkResponseScalarValue | NetworkResponseScalarValue[] {
  // N.B. empty arrays count as empty arrays of scalar fields.
  if (Array.isArray(data)) {
    // This is maybe fixed in a new version of Typescript??
    return (data as any).every((x: any) => isScalarOrEmptyArray(x));
  }
  const isScalarValue =
    typeof data === "string" ||
    typeof data === "number" ||
    typeof data === "boolean";
  return isScalarValue;
}

function isScalarButNotEmptyArray(
  data: NonNullable<NetworkResponseValue>
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
    typeof data === "string" ||
    typeof data === "number" ||
    typeof data === "boolean";
  return isScalarValue;
}

export function getParentRecordKey(
  astNode:
    | NormalizationLinkedField
    | NormalizationScalarField
    | ReaderLinkedField
    | ReaderScalarField,
  variables: { [index: string]: string }
): string {
  let parentRecordKey = astNode.fieldName;
  const fieldParameters = astNode.arguments;
  if (fieldParameters != null) {
    for (const fieldParameter of fieldParameters) {
      const { argumentName, variableName } = fieldParameter;
      const valueToUse = variables[variableName];
      parentRecordKey += `${FIRST_SPLIT_KEY}${argumentName}${SECOND_SPLIT_KEY}${valueToUse}`;
    }
  }

  return parentRecordKey;
}

function getNetworkResponseKey(
  astNode: NormalizationLinkedField | NormalizationScalarField
): string {
  let networkResponseKey = astNode.fieldName;
  const fieldParameters = astNode.arguments;
  if (fieldParameters != null) {
    for (const fieldParameter of fieldParameters) {
      const { argumentName, variableName } = fieldParameter;
      networkResponseKey += `${FIRST_SPLIT_KEY}${argumentName}${SECOND_SPLIT_KEY}${variableName}`;
    }
  }
  return networkResponseKey;
}

// an alias might be pullRequests____first___first____after___cursor
export const FIRST_SPLIT_KEY = "____";
export const SECOND_SPLIT_KEY = "___";

// Returns a key to look up an item in the store
function getDataIdOfNetworkResponse(
  parentRecordId: DataId,
  dataToNormalize: NetworkResponseObject,
  astNode: NormalizationLinkedField | NormalizationScalarField,
  variables: { [index: string]: string },
  index?: number
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
    const { argumentName, variableName } = fieldParameter;
    const valueToUse = variables[variableName];
    storeKey += `${FIRST_SPLIT_KEY}${argumentName}${SECOND_SPLIT_KEY}${valueToUse}`;
  }
  return storeKey;
}
