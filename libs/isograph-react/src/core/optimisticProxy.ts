import type {
  DataLayer,
  IsographEnvironment,
  IsographStore,
  StoreLink,
  StoreRecord,
} from './IsographEnvironment';

export function getOrInsertRecord(dataLayer: DataLayer, link: StoreLink) {
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
        let node: OptimisticLayer | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.get(storeRecord, p, storeRecord);
            if (value !== undefined) {
              return value;
            }
          }
          node = node.childNode;
        }
      },
      has(_, p) {
        let node: OptimisticLayer | null = environment.store;

        while (node !== null) {
          const storeRecord = node.data[link.__typename]?.[link.__link];
          if (storeRecord != undefined) {
            const value = Reflect.has(storeRecord, p);
            if (value) {
              return true;
            }
          }
          node = node.childNode;
        }
        return false;
      },
    },
  );
}

type BaseNode = {
  readonly kind: 'BaseNode';
  parentNode: OptimisticNode | null;
  childNode: null;
  readonly data: IsographStore;
};

type NetworkResponseNode = {
  readonly kind: 'NetworkResponseNode';
  parentNode: OptimisticNode | StartUpdateNode | null;
  childNode: OptimisticNode | StartUpdateNode;
  data: DataLayer;
};

type StartUpdateNode = {
  readonly kind: 'StartUpdateNode';
  parentNode: OptimisticNode | NetworkResponseNode | null;
  childNode: OptimisticNode | NetworkResponseNode;
  data: DataLayer;
  startUpdate: () => DataLayer;
};

type OptimisticNode = {
  readonly kind: 'OptimisticNode';
  parentNode: OptimisticNode | StartUpdateNode | NetworkResponseNode | null;
  childNode: OptimisticNode | StartUpdateNode | NetworkResponseNode | BaseNode;
  data: DataLayer;
  startUpdate: () => DataLayer;
};

export function addNetworkResponseNode(
  environment: IsographEnvironment,
  data: DataLayer,
) {
  switch (environment.store.kind) {
    case 'NetworkResponseNode':
    case 'BaseNode': {
      mergeDataLayer(environment.store.data, data);
      break;
    }
    case 'StartUpdateNode':
    case 'OptimisticNode': {
      const node: NetworkResponseNode = {
        kind: 'NetworkResponseNode',
        childNode: environment.store,
        parentNode: null,
        data,
      };
      environment.store.parentNode = node;
      environment.store = node;
      break;
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable');
    }
  }
}

function mergeDataLayer(target: DataLayer, source: DataLayer) {
  for (const typeName in source) {
    target[typeName] ??= {};
    for (const id in source[typeName]) {
      target[typeName][id] ??= {};
      Object.assign(target[typeName][id], source[typeName][id]);
    }
  }
}

export function addStartUpdateNode(
  environment: IsographEnvironment,
  startUpdate: () => DataLayer,
) {
  switch (environment.store.kind) {
    case 'BaseNode': {
      mergeDataLayer(environment.store.data, startUpdate());
      break;
    }
    case 'StartUpdateNode': {
      const prevStartUpdate = environment.store.startUpdate;

      mergeDataLayer(environment.store.data, startUpdate());

      environment.store.startUpdate = () => {
        const data = prevStartUpdate();
        mergeDataLayer(data, startUpdate());
        return data;
      };

      break;
    }
    case 'NetworkResponseNode':
    case 'OptimisticNode': {
      const node: StartUpdateNode = {
        kind: 'StartUpdateNode',
        childNode: environment.store,
        parentNode: null,
        data: startUpdate(),
        startUpdate: startUpdate,
      };
      environment.store.parentNode = node;
      environment.store = node;
      break;
    }
    default: {
      environment.store satisfies never;
    }
  }
}

export function addOptimisticNode(
  environment: IsographEnvironment,
  startUpdate: () => DataLayer,
) {
  switch (environment.store.kind) {
    case 'BaseNode':
    case 'StartUpdateNode':
    case 'NetworkResponseNode':
    case 'OptimisticNode': {
      const node: OptimisticNode = {
        kind: 'OptimisticNode',
        childNode: environment.store,
        parentNode: null,
        data: startUpdate(),
        startUpdate: startUpdate,
      };

      environment.store.parentNode = node;
      environment.store = node;

      return (data: DataLayer) => {
        replaceOptimisticNodeWithNetworkResponseNode(environment, node, data);
      };
    }
    default: {
      environment.store satisfies never;
      throw new Error('Unreachable');
    }
  }
}

function reexecuteUpdates(
  environment: IsographEnvironment,
  node: OptimisticLayer | null,
) {
  while (node && node?.kind !== 'OptimisticNode') {
    const data = 'startUpdate' in node ? node.startUpdate() : node.data;
    mergeDataLayer(environment.store.data, data);
    node = node.parentNode;
  }

  while (node !== null) {
    if ('startUpdate' in node) {
      node.data = node.startUpdate();
    }

    node.childNode = environment.store;
    environment.store.parentNode = node;
    environment.store = node;

    node = node.parentNode;

    environment.store.parentNode = null;
  }
}

function makeRootNode(environment: IsographEnvironment, node: OptimisticLayer) {
  node.parentNode = null;
  if (node.kind !== 'BaseNode') {
    environment.store.parentNode = node;
  }
  environment.store = node;
}

function replaceOptimisticNodeWithNetworkResponseNode(
  environment: IsographEnvironment,
  optimisticNode: OptimisticNode,
  data: DataLayer,
) {
  if (
    optimisticNode.childNode.kind === 'BaseNode' ||
    optimisticNode.childNode.kind === 'NetworkResponseNode'
  ) {
    mergeDataLayer(optimisticNode.childNode.data, data);

    makeRootNode(environment, optimisticNode.childNode);
    reexecuteUpdates(environment, optimisticNode.parentNode);
  } else if (optimisticNode.parentNode?.kind === 'NetworkResponseNode') {
    const networkResponseNode = optimisticNode.parentNode;
    mergeDataLayer(data, networkResponseNode.data);
    networkResponseNode.data = data;

    networkResponseNode.childNode = optimisticNode.childNode;
    optimisticNode.childNode.parentNode = networkResponseNode;

    const parentNode = optimisticNode.parentNode.parentNode;
    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, parentNode);
  } else {
    const networkResponseNode: NetworkResponseNode = {
      kind: 'NetworkResponseNode',
      data,
      childNode: optimisticNode.childNode,
      parentNode: null,
    };

    makeRootNode(environment, networkResponseNode);
    reexecuteUpdates(environment, optimisticNode.parentNode);
  }
}

export type OptimisticLayer =
  | OptimisticNode
  | NetworkResponseNode
  | StartUpdateNode
  | BaseNode;
