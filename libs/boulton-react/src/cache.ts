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
      normalizeData(networkResponse.data);
      console.log("after normalizing", store);
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

function normalizeData(data: DataType) {
  normalizeDataWithPath(data, ROOT_ID);
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

function normalizeDataWithPath(data: DataType, path: string): DataId {
  const id = data["id"] ?? path;
  const targetRecord: DataType = store[id] ?? {};
  store[id] = targetRecord;

  Object.keys(data).forEach((key) => {
    targetRecord[key] = getFieldOrNormalize(data[key], `${path}.${key}`);
  });
  return id;
}

function getFieldOrNormalize(data: DataTypeValue, path: string): DataTypeValue {
  if (typeof data === "string" || data == null) {
    return data;
  }
  if (Array.isArray(data)) {
    return data.map((item, index) =>
      getFieldOrNormalize(item, `${path}[${index}]`)
    );
  }

  const dataId = normalizeDataWithPath(data, path);
  return { __link: dataId };
}
