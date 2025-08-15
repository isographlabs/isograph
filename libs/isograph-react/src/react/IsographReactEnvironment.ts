import {
  createIsographEnvironment,
  IsographComponentFunction,
  IsographEnvironment,
  IsographNetworkFunction,
  IsographStore,
  MissingFieldHandler,
} from '../core/IsographEnvironment';
import { LogFunction, logMessage } from '../core/logging';
import { readPromise } from '../core/PromiseWrapper';
import { useReadAndSubscribe } from './useReadAndSubscribe';

export type IsographReactEnvironment = IsographEnvironment<React.FC<any>>;

export function createIsographReactEnvironment(
  store: IsographStore,
  networkFunction: IsographNetworkFunction,
  missingFieldHandler?: MissingFieldHandler | null,
  logFunction?: LogFunction | null,
): IsographReactEnvironment {
  return createIsographEnvironment(
    store,
    networkFunction,
    componentFunction,
    missingFieldHandler,
    logFunction,
  );
}

const componentFunction: IsographComponentFunction<React.FC<any>> = (
  environment,
  componentName,
  fragmentReference,
  networkRequestOptions,
  startUpdate,
) => {
  function Component(additionalRuntimeProps: { [key: string]: any }) {
    const readerWithRefetchQueries = readPromise(
      fragmentReference.readerWithRefetchQueries,
    );

    const data = useReadAndSubscribe(
      fragmentReference,
      networkRequestOptions,
      readerWithRefetchQueries.readerArtifact.readerAst,
    );

    logMessage(environment, () => ({
      kind: 'ComponentRerendered',
      componentName,
      rootLink: fragmentReference.root,
    }));

    return readerWithRefetchQueries.readerArtifact.resolver(
      {
        data,
        parameters: fragmentReference.variables,
        startUpdate: readerWithRefetchQueries.readerArtifact.hasUpdatable
          ? startUpdate
          : undefined,
      },
      additionalRuntimeProps,
    );
  }
  Component.displayName = `${componentName} (id: ${fragmentReference.root}) @component`;
  return Component;
};
