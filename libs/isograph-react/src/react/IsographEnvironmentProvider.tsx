import * as React from 'react';
import { ReactNode, createContext, useContext } from 'react';
import { type IsographEnvironment } from '../core/IsographEnvironment';

export const IsographEnvironmentContext =
  createContext<IsographEnvironment | null>(null);

export type IsographEnvironmentProviderProps = {
  readonly environment: IsographEnvironment;
  readonly children: ReactNode;
};

export function IsographEnvironmentProvider({
  environment,
  children,
}: IsographEnvironmentProviderProps): React.ReactElement {
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
