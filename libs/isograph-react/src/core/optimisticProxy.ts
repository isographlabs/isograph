import {
  callSubscriptions,
  insertEmptySetIfMissing,
  type EncounteredIds,
} from './cache';
import type {
  StoreLayerData,
  IsographEnvironment,
  BaseStoreLayerData,
  StoreLink,
  StoreRecord,
} from './IsographEnvironment';
import { logMessage } from './logging';

export function getOrInsertRecord(dataLayer: StoreLayerData, link: StoreLink) {
  const recordsById = (dataLayer[link.__typename] ??= {});
  return (recordsById[link.__link] ??= {});
}

export function readOptimisticRecord(
  environment: IsographEnvironment,
  link: StoreLink,
) {
  return new Proxy<StoreRecord>(
    {},
    {
      get(_, p) {
        let node: StoreLayer | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.get(storeRecord, p, storeRecord);
            if (value !== undefined) {
              return value;
            }
          }
          node = node.parentStoreLayer;
        }
      },
      has(_, p) {
        let node: StoreLayer | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.has(storeRecord, p);
            if (value) {
              return true;
            }
          }
          node = node.parentStoreLayer;
        }
        return false;
      },
    },
  );
}

type BaseStoreLayer = {
  readonly kind: 'BaseStoreLayer';
  childStoreLayer: OptimisticStoreLayer | null;
  parentStoreLayer: null;
  readonly data: BaseStoreLayerData;
};

type NetworkResponseStoreLayer = {
  readonly kind: 'NetworkResponseStoreLayer';
  childStoreLayer: OptimisticStoreLayer | StartUpdateStoreLayer | null;
  parentStoreLayer: OptimisticStoreLayer | StartUpdateStoreLayer;
  data: StoreLayerData;
};

export type WithEncounteredIds<T> = {
  readonly encounteredIds: EncounteredIds;
  readonly data: T;
};

type FirstUpdate = () => WithEncounteredIds<StoreLayerData>;
type DataUpdate = () => Pick<WithEncounteredIds<StoreLayerData>, 'data'>;

type StartUpdateStoreLayer = {
  readonly kind: 'StartUpdateStoreLayer';
  childStoreLayer: OptimisticStoreLayer | NetworkResponseStoreLayer | null;
  parentStoreLayer: OptimisticStoreLayer | NetworkResponseStoreLayer;
  data: StoreLayerData;
  startUpdate: DataUpdate;
};

type OptimisticStoreLayer = {
  readonly kind: 'OptimisticStoreLayer';
  childStoreLayer:
    | OptimisticStoreLayer
    | StartUpdateStoreLayer
    | NetworkResponseStoreLayer
    | null;
  parentStoreLayer:
    | OptimisticStoreLayer
    | StartUpdateStoreLayer
    | NetworkResponseStoreLayer
    | BaseStoreLayer;
  data: StoreLayerData;
  startUpdate: DataUpdate;
};

export function addNetworkResponseStoreLayer(
  environment: IsographEnvironment,
  data: StoreLayerData,
  encounteredIds: EncounteredIds,
) {
  switch (environment.store.kind) {
    case 'NetworkResponseStoreLayer':
    case 'BaseStoreLayer': {
      mergeDataLayer(environment.store.data, data);
      break;
    }
    case 'StartUpdateStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: NetworkResponseStoreLayer = {
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: environment.store,
        childStoreLayer: null,
        data,
      };
      environment.store.childStoreLayer = node;
      environment.store = node;
      break;
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable');
    }
  }

  logMessage(environment, () => ({
    kind: 'AfterNormalization',
    store: environment.store,
    encounteredIds: encounteredIds,
  }));

  callSubscriptions(environment, encounteredIds);
}

function mergeDataLayer(target: StoreLayerData, source: StoreLayerData): void {
  for (const typeName in source) {
    target[typeName] ??= {};
    for (const id in source[typeName]) {
      target[typeName][id] ??= {};
      Object.assign(target[typeName][id], source[typeName][id]);
    }
  }
}

export function addStartUpdateStoreLayer(
  environment: IsographEnvironment,
  startUpdate: FirstUpdate,
) {
  const { data, encounteredIds } = startUpdate();

  switch (environment.store.kind) {
    case 'BaseStoreLayer': {
      mergeDataLayer(environment.store.data, data);
      break;
    }
    case 'StartUpdateStoreLayer': {
      const prevStartUpdate = environment.store.startUpdate;

      mergeDataLayer(environment.store.data, data);

      environment.store.startUpdate = () => {
        const { data } = prevStartUpdate();
        mergeDataLayer(data, startUpdate().data);
        return { data };
      };

      break;
    }
    case 'NetworkResponseStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: StartUpdateStoreLayer = {
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: environment.store,
        childStoreLayer: null,
        data,
        startUpdate: startUpdate,
      };
      environment.store.childStoreLayer = node;
      environment.store = node;
      break;
    }
    default: {
      environment.store satisfies never;
    }
  }

  logMessage(environment, () => ({
    kind: 'StartUpdateComplete',
    updatedIds: encounteredIds,
  }));

  callSubscriptions(environment, encounteredIds);
}

export function addOptimisticStoreLayer(
  environment: IsographEnvironment,
  startUpdate: FirstUpdate,
) {
  const { data, encounteredIds } = startUpdate();

  switch (environment.store.kind) {
    case 'BaseStoreLayer':
    case 'StartUpdateStoreLayer':
    case 'NetworkResponseStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: OptimisticStoreLayer = {
        kind: 'OptimisticStoreLayer',
        parentStoreLayer: environment.store,
        childStoreLayer: null,
        data,
        startUpdate: startUpdate,
      };

      environment.store.childStoreLayer = node;
      environment.store = node;

      callSubscriptions(environment, encounteredIds);
      return (data: StoreLayerData) => {
        const encounteredIds: EncounteredIds = new Map();
        compareData(node.data, data, encounteredIds);
        replaceOptimisticStoreLayerWithNetworkResponseStoreLayer(
          environment,
          node,
          data,
          encounteredIds,
        );
        callSubscriptions(environment, encounteredIds);
      };
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable');
    }
  }
}

function mergeParentNodes(
  environment: IsographEnvironment,
  node:
    | OptimisticStoreLayer
    | NetworkResponseStoreLayer
    | StartUpdateStoreLayer
    | null,
  mutableEncounteredIds: EncounteredIds,
) {
  while (node && node?.kind !== 'OptimisticStoreLayer') {
    const data = 'startUpdate' in node ? node.startUpdate().data : node.data;
    compareData(node.data, data, mutableEncounteredIds);
    mergeDataLayer(environment.store.data, data);
    node = node.childStoreLayer;
  }
  return node;
}

function reexecuteUpdates(
  environment: IsographEnvironment,
  node:
    | OptimisticStoreLayer
    | NetworkResponseStoreLayer
    | StartUpdateStoreLayer
    | null,
  mutableEncounteredIds: EncounteredIds,
) {
  while (node !== null) {
    const oldData = node.data;
    if ('startUpdate' in node) {
      node.data = node.startUpdate().data;
    }
    compareData(oldData, node.data, mutableEncounteredIds);
    node.parentStoreLayer = environment.store;
    environment.store.childStoreLayer = node;
    environment.store = node;

    node = node.childStoreLayer;

    environment.store.childStoreLayer = null;
  }
}

function makeRootNode(environment: IsographEnvironment, node: StoreLayer) {
  node.childStoreLayer = null;
  environment.store = node;
}

function replaceOptimisticStoreLayerWithNetworkResponseStoreLayer(
  environment: IsographEnvironment,
  optimisticNode: OptimisticStoreLayer,
  data: StoreLayerData,
  encounteredIds: EncounteredIds,
) {
  if (optimisticNode.parentStoreLayer.kind === 'BaseStoreLayer') {
    mergeDataLayer(optimisticNode.parentStoreLayer.data, data);

    makeRootNode(environment, optimisticNode.parentStoreLayer);
    const node = mergeParentNodes(
      environment,
      optimisticNode.childStoreLayer,
      encounteredIds,
    );
    reexecuteUpdates(environment, node, encounteredIds);
  } else if (
    optimisticNode.parentStoreLayer.kind === 'NetworkResponseStoreLayer'
  ) {
    mergeDataLayer(optimisticNode.parentStoreLayer.data, data);

    makeRootNode(environment, optimisticNode.parentStoreLayer);
    reexecuteUpdates(
      environment,
      optimisticNode.childStoreLayer,
      encounteredIds,
    );
  } else if (
    optimisticNode.childStoreLayer?.kind === 'NetworkResponseStoreLayer'
  ) {
    const networkResponseNode = optimisticNode.childStoreLayer;
    mergeDataLayer(data, networkResponseNode.data);
    networkResponseNode.data = data;

    networkResponseNode.parentStoreLayer = optimisticNode.parentStoreLayer;
    optimisticNode.parentStoreLayer.childStoreLayer = networkResponseNode;

    const childStoreLayer = optimisticNode.childStoreLayer.childStoreLayer;
    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, childStoreLayer, encounteredIds);
  } else {
    const networkResponseNode: NetworkResponseStoreLayer = {
      kind: 'NetworkResponseStoreLayer',
      data,
      parentStoreLayer: optimisticNode.parentStoreLayer,
      childStoreLayer: null,
    };

    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(
      environment,
      optimisticNode.childStoreLayer,
      encounteredIds,
    );
  }
}

export type StoreLayer =
  | OptimisticStoreLayer
  | NetworkResponseStoreLayer
  | StartUpdateStoreLayer
  | BaseStoreLayer;

function compareData(
  oldData: StoreLayerData,
  newData: StoreLayerData,
  encounteredIds: EncounteredIds,
) {
  if (oldData === newData) {
    for (const [typeName, ids] of encounteredIds.entries()) {
      for (const id of ids) {
        encounteredIds.get(typeName)?.delete(id);
      }
    }
    return;
  }
  for (const [typeName, records] of Object.entries(newData)) {
    if (!records) {
      continue;
    }
    outer: for (const [id, record] of Object.entries(records)) {
      if (!record) {
        continue;
      }

      for (const [recordKey, recordValue] of Object.entries(record)) {
        // TODO: compare links, compare arrays
        if (recordValue !== oldData[typeName]?.[id]?.[recordKey]) {
          const set = insertEmptySetIfMissing(encounteredIds, typeName);
          set.add(id);
          continue outer;
        }
      }

      encounteredIds.get(typeName)?.delete(id);
    }
  }
}
