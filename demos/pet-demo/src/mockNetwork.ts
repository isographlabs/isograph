import type {
  IsographNetworkFunction,
  JsonObject,
  Variables,
} from '@isograph/runtime';
import homeRouteResponse from '../fixtures/Query.HomeRoute.json';
import petDetailRouteResponses from '../fixtures/Query.PetDetailRoute.json';
import { HomeRouteEntrypoint as homeRoute } from './HomeRoute';
import { PetDetailRouteEntrypoint as petDetailRoute } from './PetDetailRoute';
import { SetTaglineEntrypoint as setTagline } from './SetTagline';

/** `Query.PetDetailRoute`'s response for each pet id the fixture has. */
const petDetailRouteResponseById: ReadonlyMap<string, JsonObject> = new Map(
  Object.entries(petDetailRouteResponses),
);

/**
 * Answers `Query.HomeRoute` and `Query.PetDetailRoute` from the fixture responses, and
 * `Mutation.SetTagline` with the pet and tagline it was sent. Rejects anything else.
 */
export const mockNetworkFunction: IsographNetworkFunction = (
  operation,
  variables,
) => {
  const id = variables['id'];
  const tagline = variables['tagline'];
  if (operation.text === homeRoute.queryText) {
    return Promise.resolve(homeRouteResponse);
  }
  if (operation.text === petDetailRoute.queryText && typeof id === 'string') {
    const response = petDetailRouteResponseById.get(id);
    return response === undefined
      ? noResponse(operation.text, variables)
      : Promise.resolve(response);
  }
  if (
    operation.text === setTagline.queryText &&
    typeof id === 'string' &&
    typeof tagline === 'string'
  ) {
    return Promise.resolve({
      set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c: {
        __typename: 'SetPetTaglineResponse',
        pet: { __typename: 'Pet', id, tagline },
      },
    });
  }
  return noResponse(operation.text, variables);
};

function noResponse(
  queryText: string,
  variables: Variables,
): Promise<JsonObject> {
  return Promise.reject(
    new Error(
      `The mock network has no response for ${queryText} with ${JSON.stringify(variables)}`,
    ),
  );
}
