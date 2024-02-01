import { ReactNode, createContext, useContext } from 'react';
import * as React from 'react';
import { subscribe } from './cache';
import { ParentCache } from '@isograph/isograph-react-disposable-state';

export const IsographEnvironmentContext =
  createContext<IsographEnvironment | null>(null);

type ComponentName = string;
type StringifiedArgs = string;
type ComponentCache = {
  [key: DataId]: {
    [key: ComponentName]: { [key: StringifiedArgs]: React.FC<any> };
  };
};

export type Subscriptions = Set<() => void>;
type SuspenseCache = { [index: string]: ParentCache<any> };

export type IsographEnvironment = {
  store: IsographStore;
  networkFunction: IsographNetworkFunction;
  missingFieldHandler: MissingFieldHandler | null;
  componentCache: ComponentCache;
  subscriptions: Subscriptions;
  suspenseCache: SuspenseCache;
};

export type MissingFieldHandler = (
  storeRecord: StoreRecord,
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null,
) => Link | undefined;

export type IsographNetworkFunction = (
  queryText: string,
  variables: object,
) => Promise<any>;

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

export const ROOT_ID: DataId & '__ROOT' = '__ROOT';

export type IsographStore = {
  [index: DataId]: StoreRecord | null;
  __ROOT: StoreRecord;
};

export type IsographEnvironmentProviderProps = {
  environment: IsographEnvironment;
  children: ReactNode;
};

export function IsographEnvironmentProvider({
  environment,
  children,
}: IsographEnvironmentProviderProps) {
  const [, setState] = React.useState<object | void>();
  React.useEffect(() => {
    return subscribe(environment, () => setState({}));
  }, []);

  return (
    <IsographEnvironmentContext.Provider value={environment}>
      {children}
    </IsographEnvironmentContext.Provider>
  );
}

export function useIsographEnvironment(): IsographEnvironment {
  const context = useContext(IsographEnvironmentContext);
  if (context == null) {
    throw new Error(
      'Unexpected null environment context. Make sure to render ' +
        'this component within an IsographEnvironmentProvider component',
    );
  }
  return context;
}

export function createIsographEnvironment(
  store: IsographStore,
  networkFunction: IsographNetworkFunction,
  missingFieldHandler?: MissingFieldHandler,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    missingFieldHandler: missingFieldHandler ?? null,
    componentCache: {},
    subscriptions: new Set(),
    suspenseCache: {},
  };
}

export function createIsographStore() {
  return {
    [ROOT_ID]: {},
  };
}
