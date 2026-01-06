import { type NotImplemented____link__output_type } from '../../NotImplemented/__link/output_type';
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
      /**
A discriminant for the TopLevelField type
      */
      readonly __typename: "TopLevelField",
    } | null),
    readonly namable: ({
      /**
A discriminant for the Namable type
      */
      readonly __typename: "Pet",
    } | null),
    readonly notImplemented: ({
      /**
A discriminant for the NotImplemented type
      */
      readonly __typename: never,
      /**
A store Link for the NotImplemented type.
      */
      readonly __link: NotImplemented____link__output_type,
    } | null),
  },
  readonly parameters: Query__PetDetailDeferredRoute__parameters,
};
