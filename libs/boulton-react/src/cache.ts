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

export function getOrCreateCacheForUrl<T extends object>(
  query_text: string,
  variables: object
): ParentCache<PromiseWrapper<T>> {
  const factory: Factory<PromiseWrapper<T>> = () =>
    makeNetworkRequest<T>(query_text, variables);
  return getOrCreateCache<PromiseWrapper<T>>(query_text, factory);
}

export function makeNetworkRequest<T extends object>(
  query_text: string,
  variables: object
): ItemCleanupPair<PromiseWrapper<T>> {
  let promise: Promise<T> = fetch("http://localhost:4000/graphql", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: query_text, variables }),
  })
    .then((response) => response.json())
    .then((networkResponse) => {
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
    const actualKey = HACK_map_key(networkResponseKey, variables);
    targetRecord[actualKey] = getFieldOrNormalize(
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
function HACK_map_key(
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
