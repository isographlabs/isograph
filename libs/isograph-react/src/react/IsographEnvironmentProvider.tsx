import * as React from 'react';
import { createContext, ReactNode, useContext } from 'react';
import { IsographReactEnvironment } from './IsographReactEnvironment';

export const IsographEnvironmentContext =
  createContext<IsographReactEnvironment | null>(null);

export type IsographEnvironmentProviderProps = {
  readonly environment: IsographReactEnvironment;
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

export function useIsographEnvironment(): IsographReactEnvironment {
  const context = useContext(IsographEnvironmentContext);
  if (context == null) {
    throw new Error(
      'Unexpected null environment context. Make sure to render ' +
        'this component within an IsographEnvironmentProvider component',
    );
  }
  return context;
}
