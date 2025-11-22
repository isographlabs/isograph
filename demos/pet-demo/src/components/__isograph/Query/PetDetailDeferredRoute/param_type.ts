import { type Pet__PetDetailDeferredRouteInnerComponent__output_type } from '../../Pet/PetDetailDeferredRouteInnerComponent/output_type';
import type { Query__PetDetailDeferredRoute__parameters } from './parameters_type';

export type Query__PetDetailDeferredRoute__param = {
  readonly data: {
    /**
Fetch a pet by id
    */
    readonly pet: ({
      readonly PetDetailDeferredRouteInnerComponent: Pet__PetDetailDeferredRouteInnerComponent__output_type,
    } | null),
    /**
Don't use this field. It's for testing.
    */
    readonly topLevelField: ({
      readonly __typename: 'TopLevelField',
    } | null),
  },
  readonly parameters: Query__PetDetailDeferredRoute__parameters,
};
