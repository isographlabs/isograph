import type {
  DataId,
  DataTypeValue,
  IsographEnvironment,
  IsographStore,
  StoreRecord,
  TypeName,
} from './IsographEnvironment';

function createLayerProxy<T>(
  object: {
    [key: string]: T | null;
  },
  optimisticObject: {
    [key: string]: T | null;
  },
  getter: (
    value: T | null | undefined,
    optimisticValue: T | undefined,
    p: string,
  ) => T | null | undefined,
): {
  [key: string]: T | null;
} {
  return new Proxy(optimisticObject, {
    get(target, p: string) {
      let optimisticValue = target[p];

      if (optimisticValue === null) {
        return optimisticValue;
      }

      const value = object[p];

      if (optimisticValue === undefined && value == null) {
        return value;
      }

      return getter(value, optimisticValue, p);
    },
    has(target, p) {
      return Reflect.has(target, p) || Reflect.has(object, p);
    },
    ownKeys(target) {
      const merged = {
        ...object,
        ...target,
      };
      return Reflect.ownKeys(merged);
    },
    set(target, p: string, value: any) {
      return Reflect.set(target, p, value);
    },
    getOwnPropertyDescriptor(target, p: string) {
      return (
        Reflect.getOwnPropertyDescriptor(target, p) ??
        Reflect.getOwnPropertyDescriptor(object, p)
      );
    },
  });
}

export function createOptimisticProxy(
  store: IsographStore,
  optimisticLayer: OptimisticLayer,
): OptimisticLayer {
  return createLayerProxy(
    store,
    optimisticLayer,
    (recordsById, optimisticRecordsById, p) => {
      optimisticRecordsById = optimisticLayer[p] ??= {};
      return createLayerProxy(
        recordsById ?? {},
        optimisticRecordsById,
        (storeRecord, optimisticStoreRecord, p) => {
          optimisticStoreRecord = optimisticRecordsById[p] ??= {};
          return createLayerProxy(
            storeRecord ?? {},
            optimisticStoreRecord,
            (value, optimisticValue) =>
              optimisticValue === undefined ? value : optimisticValue,
          );
        },
      );
    },
  );
}

export type OptimisticLayer = {
  [index: TypeName]: {
    [index: DataId]: StoreRecord | null;
  } | null;
};

export function mergeOptimisticLayer(environment: IsographEnvironment): void {
  for (const [typeName, patchById] of Object.entries(
    environment.optimisticLayer,
  )) {
    let recordById = environment.store[typeName];

    if (patchById === null) {
      environment.store[typeName] = null;
      continue;
    }
    recordById = environment.store[typeName] ??= {};

    for (const [recordId, patch] of Object.entries(patchById)) {
      const data = recordById[recordId];

      if (patch == null || data == null) {
        recordById[recordId] = patch;
      } else {
        Object.assign(data, patch);
      }
    }
  }

  resetOptimisticLayer(environment);
}

export function resetOptimisticLayer(environment: IsographEnvironment) {
  for (const key in environment.optimisticLayer) {
    delete environment.optimisticLayer[key];
  }
}
