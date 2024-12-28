import { RetainedQuery } from '../core/garbageCollection';
import { ROOT_ID } from '../core/IsographEnvironment';
import { iso } from './__isograph/iso';

// TODO investigate why this can't be in garbageCollection.test.ts without
// typescript incorrectly thinking it is referenced in its own initializer
export const nodeField = iso(`
  field Query.nodeField($id: ID!) {
    node(id: $id) {
      id
    }
  }
`)(() => {});
const nodeFieldEntrypoint = iso(`entrypoint Query.nodeField`);
export const nodeFieldRetainedQuery: RetainedQuery = {
  normalizationAst:
    nodeFieldEntrypoint.networkRequestInfo.normalizationAst.selections,
  variables: { id: 0 },
  root: { __link: ROOT_ID, __typename: 'Query' },
};
