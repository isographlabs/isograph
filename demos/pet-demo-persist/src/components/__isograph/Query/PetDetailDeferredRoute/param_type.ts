import { type Pet__PetDetailDeferredRouteInnerComponent__output_type } from '../../Pet/PetDetailDeferredRouteInnerComponent/output_type';
import type { Query__PetDetailDeferredRoute__parameters } from './parameters_type';

export type Query__PetDetailDeferredRoute__param = {
  readonly data: {
    readonly pet: ({
      readonly PetDetailDeferredRouteInnerComponent: Pet__PetDetailDeferredRouteInnerComponent__output_type,
    } | null),
    readonly topLevelField: ({
      readonly __typename: string,
    } | null),
  },
  readonly parameters: Query__PetDetailDeferredRoute__parameters,
};
