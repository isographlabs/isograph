import {
  createIsographEnvironmentCore,
  type BaseStoreLayerData,
  type IsographNetworkFunction,
  type MissingFieldHandler,
} from '../core/IsographEnvironment';
import type { LogFunction } from '../core/logging';
import { componentFunction } from './useReadAndSubscribe';

export function createIsographEnvironment(
  baseStoreLayerData: BaseStoreLayerData,
  networkFunction: IsographNetworkFunction,
  missingFieldHandler?: MissingFieldHandler | null,
  logFunction?: LogFunction | null,
) {
  return createIsographEnvironmentCore(
    baseStoreLayerData,
    networkFunction,
    componentFunction,
    missingFieldHandler,
    logFunction,
  );
}
