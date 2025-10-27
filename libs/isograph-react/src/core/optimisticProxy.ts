import {
  callSubscriptions,
  insertEmptySetIfMissing,
  type EncounteredIds,
} from './cache';
import type {
  BaseStoreLayerData,
  IsographEnvironment,
  StoreLayerData,
  StoreLink,
  StoreRecord,
} from './IsographEnvironment';
import { logMessage } from './logging';

export function getOrInsertRecord(
  dataLayer: StoreLayerData,
  link: StoreLink,
): StoreRecord {
  const recordsById = (dataLayer[link.__typename] ??= {});
  return (recordsById[link.__link] ??= {});
}

export function readOptimisticRecord(
  storeLayer: StoreLayer,
  link: StoreLink,
): StoreRecord {
  return new Proxy<StoreRecord>(
    {},
    {
      get(_, p) {
        let node: StoreLayer | null = storeLayer;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.get(storeRecord, p);
            if (value !== undefined) {
              return value;
            }
          }
          node = node.parentStoreLayer;
        }
      },
      has(_, p) {
        let node: StoreLayer | null = storeLayer;

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
      set(_, p, newValue) {
        return Reflect.set(
          getOrInsertRecord(storeLayer.data, link),
          p,
          newValue,
        );
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

export type FirstUpdate = (storeLayer: StoreLayer) => EncounteredIds;
type DataUpdate = (storeLayer: StoreLayer) => void;

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
  normalizeData: FirstUpdate,
): void {
  let encounteredIds: EncounteredIds;
  switch (environment.store.kind) {
    case 'NetworkResponseStoreLayer':
    case 'BaseStoreLayer': {
      encounteredIds = normalizeData(environment.store);
      break;
    }
    case 'StartUpdateStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: NetworkResponseStoreLayer = {
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: environment.store,
        childStoreLayer: null,
        data: {},
      };
      environment.store.childStoreLayer = node;
      environment.store = node;

      encounteredIds = normalizeData(node);
      break;
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
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
): void {
  let encounteredIds: EncounteredIds;

  switch (environment.store.kind) {
    case 'BaseStoreLayer': {
      encounteredIds = startUpdate(environment.store);
      break;
    }
    case 'StartUpdateStoreLayer': {
      const node = environment.store;

      const prevStartUpdate = node.startUpdate;
      node.startUpdate = () => {
        prevStartUpdate(node);
        startUpdate(node);
      };

      encounteredIds = startUpdate(node);
      break;
    }
    case 'NetworkResponseStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: StartUpdateStoreLayer = {
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: environment.store,
        childStoreLayer: null,
        data: {},
        startUpdate: startUpdate,
      };
      environment.store.childStoreLayer = node;
      environment.store = node;

      encounteredIds = startUpdate(node);
      break;
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
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
  switch (environment.store.kind) {
    case 'BaseStoreLayer':
    case 'StartUpdateStoreLayer':
    case 'NetworkResponseStoreLayer':
    case 'OptimisticStoreLayer': {
      const data = {};

      const node: OptimisticStoreLayer = {
        kind: 'OptimisticStoreLayer',
        parentStoreLayer: environment.store,
        childStoreLayer: null,
        data,
        startUpdate: startUpdate,
      };

      const encounteredIds = startUpdate(node);

      environment.store.childStoreLayer = node;
      environment.store = node;

      callSubscriptions(environment, encounteredIds);
      return (
        normalizeData: (storeLayer: StoreLayer) => EncounteredIds,
      ): void => {
        replaceOptimisticStoreLayerWithNetworkResponseStoreLayer(
          environment,
          node,
          normalizeData,
        );
      };
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
}

function mergeChildNodes(
  node: StoreLayer | null,
  baseNode: BaseStoreLayer | NetworkResponseStoreLayer,
): OptimisticStoreLayer | null {
  while (node && node.kind !== 'OptimisticStoreLayer') {
    mergeDataLayer(baseNode.data, node.data);
    node = node.childStoreLayer;
  }
  return node;
}

function reexecuteUpdates(
  node:
    | OptimisticStoreLayer
    | NetworkResponseStoreLayer
    | StartUpdateStoreLayer
    | null,
  oldData: StoreLayerData,
  newData: StoreLayerData,
): void {
  while (node !== null) {
    mergeDataLayer(oldData, node.data);
    if ('startUpdate' in node) {
      node.data = {};
      node.startUpdate(node);
    }
    mergeDataLayer(newData, node.data);

    node = node.childStoreLayer;
  }
}

function replaceOptimisticStoreLayerWithNetworkResponseStoreLayer(
  environment: IsographEnvironment,
  optimisticNode: OptimisticStoreLayer,
  normalizeData: (storeLayer: StoreLayer) => void,
): void {
  const oldData = optimisticNode.data;
  //  we cannot replace optimistic node with network response directly
  // because of the types so we have to:
  //  1. reset the optimistic node
  optimisticNode.data = {};
  // 2. append the network response as child
  const networkResponseNode: NetworkResponseStoreLayer = {
    kind: 'NetworkResponseStoreLayer',
    data: {},
    parentStoreLayer: optimisticNode,
    childStoreLayer: null,
  };
  normalizeData(networkResponseNode);

  let childNode = optimisticNode.childStoreLayer;

  if (childNode?.kind === 'NetworkResponseStoreLayer') {
    mergeDataLayer(networkResponseNode.data, childNode.data);
    mergeDataLayer(oldData, childNode.data);
    childNode = childNode.childStoreLayer;
  }
  const newData = structuredClone(networkResponseNode.data);

  networkResponseNode.childStoreLayer = childNode;
  if (childNode) {
    childNode.parentStoreLayer = networkResponseNode;
  } else {
    environment.store = networkResponseNode;
  }
  optimisticNode.childStoreLayer = networkResponseNode;

  // reexecute all updates after the network response
  reexecuteUpdates(networkResponseNode.childStoreLayer, oldData, newData);
  // merge the child nodes if possible and remove them or remove the optimistic node
  if (optimisticNode.parentStoreLayer.kind === 'BaseStoreLayer') {
    const childOptimisticNode = mergeChildNodes(
      optimisticNode.childStoreLayer,
      optimisticNode.parentStoreLayer,
    );

    optimisticNode.parentStoreLayer.childStoreLayer = childOptimisticNode;
    if (childOptimisticNode) {
      childOptimisticNode.parentStoreLayer = optimisticNode.parentStoreLayer;
    } else {
      environment.store = optimisticNode.parentStoreLayer;
    }
  } else if (
    optimisticNode.parentStoreLayer.kind == 'NetworkResponseStoreLayer'
  ) {
    mergeDataLayer(
      optimisticNode.parentStoreLayer.data,
      networkResponseNode.data,
    );
    optimisticNode.parentStoreLayer.childStoreLayer =
      networkResponseNode.childStoreLayer;
    if (networkResponseNode.childStoreLayer) {
      networkResponseNode.childStoreLayer.parentStoreLayer =
        optimisticNode.parentStoreLayer;
    } else {
      environment.store = optimisticNode.parentStoreLayer;
    }
  } else {
    optimisticNode.parentStoreLayer.childStoreLayer = networkResponseNode;
    networkResponseNode.parentStoreLayer = optimisticNode.parentStoreLayer;
  }

  let encounteredIds: EncounteredIds = new Map();
  compareData(oldData, newData, encounteredIds);
  callSubscriptions(environment, encounteredIds);
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
): void {
  const oldDataTypeNames = new Set(Object.keys(oldData));
  const newDataTypeNames = new Set(Object.keys(newData));

  for (const oldTypeName of oldDataTypeNames.difference(newDataTypeNames)) {
    const set = insertEmptySetIfMissing(encounteredIds, oldTypeName);
    for (const id in oldData[oldTypeName]) {
      set.add(id);
    }
  }

  for (const [typeName, newRecords] of Object.entries(newData)) {
    if (!newRecords) {
      continue;
    }
    const oldRecords = oldData[typeName];
    outer: for (const [id, newRecord] of Object.entries(newRecords)) {
      if (!newRecord) {
        continue;
      }
      const oldRecord = oldRecords?.[id];

      for (const [recordKey, newRecordValue] of Object.entries(newRecord)) {
        // TODO: compare links, compare arrays
        if (newRecordValue !== oldRecord?.[recordKey]) {
          const set = insertEmptySetIfMissing(encounteredIds, typeName);
          set.add(id);
          continue outer;
        }
      }

      encounteredIds.get(typeName)?.delete(id);
    }
  }
}
