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
          if (storeRecord != null) {
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
          if (storeRecord != null) {
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

export type BaseStoreLayer = {
  readonly kind: 'BaseStoreLayer';
  childStoreLayer: OptimisticStoreLayer | null;
  readonly parentStoreLayer: null;
  readonly data: BaseStoreLayerData;
};

export type NetworkResponseStoreLayer = {
  readonly kind: 'NetworkResponseStoreLayer';
  childStoreLayer: OptimisticStoreLayer | StartUpdateStoreLayer | null;
  parentStoreLayer: OptimisticStoreLayer | StartUpdateStoreLayer;
  readonly data: StoreLayerData;
};

export type DataUpdate<TStoreLayer extends StoreLayer> = (
  storeLayer: TStoreLayer,
) => void;

export type StartUpdateStoreLayer = {
  readonly kind: 'StartUpdateStoreLayer';
  childStoreLayer: OptimisticStoreLayer | NetworkResponseStoreLayer | null;
  parentStoreLayer: OptimisticStoreLayer | NetworkResponseStoreLayer;
  data: StoreLayerData;
  startUpdate: DataUpdate<StartUpdateStoreLayer | BaseStoreLayer>;
};

export type OptimisticStoreLayer = {
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
  readonly startUpdate: DataUpdate<OptimisticStoreLayer>;
};

export function addNetworkResponseStoreLayer(
  parent: StoreLayer,
  normalizeData: DataUpdate<NetworkResponseStoreLayer | BaseStoreLayer>,
): StoreLayer {
  switch (parent.kind) {
    case 'NetworkResponseStoreLayer':
    case 'BaseStoreLayer': {
      normalizeData(parent);
      return parent;
    }
    case 'StartUpdateStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: NetworkResponseStoreLayer = {
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
      };
      parent.childStoreLayer = node;

      normalizeData(node);
      return node;
    }
    default: {
      parent satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
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
  parent: StoreLayer,
  startUpdate: StartUpdateStoreLayer['startUpdate'],
): StoreLayer {
  switch (parent.kind) {
    case 'BaseStoreLayer': {
      startUpdate(parent);
      return parent;
    }
    case 'StartUpdateStoreLayer': {
      const node = parent;

      const prevStartUpdate = node.startUpdate;
      node.startUpdate = () => {
        prevStartUpdate(node);
        startUpdate(node);
      };

      startUpdate(node);
      return node;
    }
    case 'NetworkResponseStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: StartUpdateStoreLayer = {
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
        startUpdate: startUpdate,
      };
      parent.childStoreLayer = node;

      startUpdate(node);
      return node;
    }
    default: {
      parent satisfies never;
      throw new Error('Unreachable. This is a bug in Isograph.');
    }
  }
}

export function addOptimisticStoreLayer(
  parent: StoreLayer,
  startUpdate: OptimisticStoreLayer['startUpdate'],
) {
  switch (parent.kind) {
    case 'BaseStoreLayer':
    case 'StartUpdateStoreLayer':
    case 'NetworkResponseStoreLayer':
    case 'OptimisticStoreLayer': {
      const node: OptimisticStoreLayer = {
        kind: 'OptimisticStoreLayer',
        parentStoreLayer: parent,
        childStoreLayer: null,
        data: {},
        startUpdate: startUpdate,
      };

      startUpdate(node);
      parent.childStoreLayer = node;

      return {
        node,
        revert: (
          environment: IsographEnvironment,
          normalizeData: (storeLayer: StoreLayer) => EncounteredIds,
        ): void => {
          replaceOptimisticStoreLayerWithNetworkResponseStoreLayer(
            environment,
            node,
            normalizeData,
          );
        },
      };
    }
    default: {
      parent satisfies never;
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
  initialNode: OptimisticStoreLayer | StartUpdateStoreLayer | null,
  oldData: StoreLayerData,
  newData: StoreLayerData,
): void {
  let node:
    | OptimisticStoreLayer
    | NetworkResponseStoreLayer
    | StartUpdateStoreLayer
    | null = initialNode;
  while (node !== null) {
    mergeDataLayer(oldData, node.data);
    switch (node.kind) {
      case 'NetworkResponseStoreLayer':
        break;
      case 'StartUpdateStoreLayer': {
        node.data = {};
        node.startUpdate(node);
        break;
      }
      case 'OptimisticStoreLayer': {
        node.data = {};
        node.startUpdate(node);
        break;
      }
      default: {
        node satisfies never;
        throw new Error('Unreachable. This is a bug in Isograph.');
      }
    }
    mergeDataLayer(newData, node.data);

    node = node.childStoreLayer;
  }
}

function setChildOfNode<TStoreLayer extends StoreLayer>(
  environment: IsographEnvironment,
  node: TStoreLayer,
  newChild: TStoreLayer['childStoreLayer'],
) {
  node.childStoreLayer = newChild;
  if (newChild !== null) {
    newChild.parentStoreLayer = node;
  } else {
    environment.store = node;
  }
}

function replaceOptimisticStoreLayerWithNetworkResponseStoreLayer(
  environment: IsographEnvironment,
  optimisticNode: OptimisticStoreLayer,
  normalizeData: (storeLayer: StoreLayerWithData) => void,
): void {
  const oldData = optimisticNode.data;
  // we cannot replace the optimistic node with the network response directly
  // because of the types so we have to:
  // 1. reset the optimistic node
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

  setChildOfNode(environment, networkResponseNode, childNode);
  optimisticNode.childStoreLayer = networkResponseNode;

  // reexecute all updates after the network response
  reexecuteUpdates(networkResponseNode.childStoreLayer, oldData, newData);
  // merge the child nodes if possible and remove them or remove the optimistic node
  if (optimisticNode.parentStoreLayer.kind === 'BaseStoreLayer') {
    const childOptimisticNode = mergeChildNodes(
      optimisticNode.childStoreLayer,
      optimisticNode.parentStoreLayer,
    );

    setChildOfNode(
      environment,
      optimisticNode.parentStoreLayer,
      childOptimisticNode,
    );
  } else if (
    optimisticNode.parentStoreLayer.kind === 'NetworkResponseStoreLayer'
  ) {
    mergeDataLayer(
      optimisticNode.parentStoreLayer.data,
      networkResponseNode.data,
    );

    setChildOfNode(
      environment,
      optimisticNode.parentStoreLayer,
      networkResponseNode.childStoreLayer,
    );
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

export type StoreLayerWithData = BaseStoreLayer | NetworkResponseStoreLayer;

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
    if (newRecords == null) {
      continue;
    }
    const oldRecords = oldData[typeName];
    outer: for (const [id, newRecord] of Object.entries(newRecords)) {
      if (newRecord == null) {
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
