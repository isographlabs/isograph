import {
  Factory,
  ItemCleanupPair,
  ParentCache,
} from "@boulton/react-disposable-state";
import { PromiseWrapper, wrapPromise } from "./PromiseWrapper";

const cache: { [index: string]: ParentCache<any> } = {};

function getOrCreateCache<T>(
  index: string,
  factory: Factory<T>
): ParentCache<T> {
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
    return value.map(stableCopy);
  }
  const keys = Object.keys(value).sort();
  const stable: { [index: string]: any } = {};
  for (let i = 0; i < keys.length; i++) {
    stable[keys[i]] = stableCopy(value[keys[i]]);
  }
  return stable as any;
}

export function getOrCreateCacheForUrl<T>(
  queryText: string,
  variables: object
): ParentCache<PromiseWrapper<T>> {
  const factory: Factory<PromiseWrapper<T>> = () =>
    makeNetworkRequest<T>(queryText, variables);
  return getOrCreateCache<PromiseWrapper<T>>(
    queryText + JSON.stringify(stableCopy(variables)),
    factory
  );
}

let network: ((queryText: string, variables: object) => Promise<any>) | null;

// This is a hack until we store this in context somehow
export function setNetwork(newNetwork: typeof network) {
  network = newNetwork;
}

function makeNetworkRequest<T>(
  queryText: string,
  variables: object
): ItemCleanupPair<PromiseWrapper<T>> {
  if (network == null) {
    throw new Error("Network must be set before makeNetworkRequest is called");
  }

  const promise = network(queryText, variables).then((networkResponse) => {
    normalizeData(networkResponse.data, variables);
    console.log("after normalizing", JSON.stringify(store, null, 4));
    return networkResponse.data;
  });

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
  | string
  | undefined
  | DataId
  | Link
  | DataTypeValue[];

export type DataType = {
  [index: DataId | string]: DataTypeValue;
  id?: DataId;
};
export type DataId = string;

export const store: { [index: DataId]: DataType } = {};

export const ROOT_ID = "ROOT";

function normalizeData(data: DataType, variables: Object) {
  normalizeDataWithPath(data, ROOT_ID, variables);
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

function normalizeDataWithPath(
  data: DataType,
  path: string,
  variables: { [index: string]: string }
): DataId {
  const id = data["id"] ?? path;
  const targetRecord: DataType = store[id] ?? {};
  store[id] = targetRecord;

  Object.keys(data).forEach((networkResponseKey) => {
    const storeKey = HACK_get_store_key(networkResponseKey, variables);
    targetRecord[storeKey] = getFieldOrNormalize(
      data[networkResponseKey],
      `${path}.${networkResponseKey}`,
      variables
    );
  });
  return id;
}

function getFieldOrNormalize(
  data: DataTypeValue,
  path: string,
  variables: { [index: string]: string }
): DataTypeValue {
  if (typeof data === "string" || data == null) {
    return data;
  }
  if (Array.isArray(data)) {
    return data.map((item, index) =>
      getFieldOrNormalize(item, `${path}[${index}]`, variables)
    );
  }

  const dataId = normalizeDataWithPath(data, path, variables);
  return { __link: dataId };
}

/// Fields that use variables have aliases like nameOfField__fieldName1_variableName2__...
/// so e.g. node(id: $ID) becomes node__id_ID. Here, we map that back to node__id_4
/// for writing to the store.
function HACK_get_store_key(
  networkResponseKey: string,
  // {ID: "4"} and the like
  variablesToValues: { [index: string]: string }
): string {
  const parts = networkResponseKey.split("__");
  let fieldName = parts[0];

  // {id: "ID"} and the like
  const fieldArgToUsedVariable: { [index: string]: string } = {};
  for (const variable_key_val of parts.slice(1)) {
    const [fieldArgName, usedVariable] = variable_key_val.split("_");
    fieldArgToUsedVariable[fieldArgName] = usedVariable;
  }

  // {id: 4} and the like
  const fieldArgToValue: { [index: string]: string } = {};
  for (const fieldArgName in fieldArgToUsedVariable) {
    const usedVariable = fieldArgToUsedVariable[fieldArgName];
    if (variablesToValues[usedVariable] == null) {
      throw new Error(
        `Variable ${fieldArgName} used in ${networkResponseKey} but not provided in variables`
      );
    }
    fieldArgToValue[fieldArgName] = variablesToValues[usedVariable];
  }

  const sortedFields = Object.entries(fieldArgToValue).sort((a, b) =>
    a[0].localeCompare(b[0])
  );

  for (const [fieldArgName, value] of sortedFields) {
    fieldName += `__${fieldArgName}_${value}`;
  }

  return fieldName;
}
