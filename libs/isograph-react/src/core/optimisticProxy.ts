import type {
  DataId,
  IsographEnvironment,
  IsographStore,
  StoreRecord,
  TypeName,
} from './IsographEnvironment';

type RecordsById = {
  [index: DataId]: StoreRecord | null;
};

export function createOptimisticRecord(
  storeRecord: StoreRecord,
  optimisticStoreRecord: StoreRecord,
): StoreRecord {
  return new Proxy(optimisticStoreRecord, {
    get(target, p: string) {
      const optimisticValue = target[p];

      if (optimisticValue === undefined) {
        return storeRecord[p];
      }
      return optimisticValue;
    },
    has(target, p) {
      return Reflect.has(target, p) || Reflect.has(storeRecord, p);
    },
    ownKeys(target) {
      const merged = {
        ...storeRecord,
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
        Reflect.getOwnPropertyDescriptor(storeRecord, p)
      );
    },
  });
}

export function createOptimisticRecordsById(
  recordsById: RecordsById,
  optimisticRecordsById: RecordsById,
): RecordsById {
  return new Proxy(optimisticRecordsById, {
    get(target, p: string) {
      let optimisticStoreRecord = target[p];

      if (optimisticStoreRecord === null) {
        return optimisticStoreRecord;
      }

      const storeRecord = recordsById[p];

      if (storeRecord == null) {
        return storeRecord;
      }

      optimisticStoreRecord = target[p] ??= {};

      return createOptimisticRecord(storeRecord, optimisticStoreRecord);
    },
    has(target, p) {
      return Reflect.has(target, p) || Reflect.has(recordsById, p);
    },
    ownKeys(target) {
      const merged = {
        ...recordsById,
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
        Reflect.getOwnPropertyDescriptor(recordsById, p)
      );
    },
  });
}

export function createOptimisticProxy(
  store: IsographStore,
  optimisticLayer: OptimisticLayer,
): OptimisticLayer {
  return new Proxy(optimisticLayer, {
    get(target, p: string) {
      let optimisticRecordsById = target[p];

      if (optimisticRecordsById === null) {
        return optimisticRecordsById;
      }

      const recordsById = store[p];

      if (recordsById == null) {
        return recordsById;
      }

      optimisticRecordsById = target[p] ??= {};

      return createOptimisticRecordsById(recordsById, optimisticRecordsById);
    },
    has(target, p) {
      return Reflect.has(target, p) || Reflect.has(store, p);
    },
    ownKeys(target) {
      const merged = {
        ...store,
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
        Reflect.getOwnPropertyDescriptor(store, p)
      );
    },
  });
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
