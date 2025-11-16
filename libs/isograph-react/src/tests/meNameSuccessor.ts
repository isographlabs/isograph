import type { RetainedQuery } from '../core/garbageCollection';
import { ROOT_ID } from '../core/IsographEnvironment';
import { wrapResolvedValue } from '../core/PromiseWrapper';
import { iso } from './__isograph/iso';

export const meNameField = iso(`
  field Query.meNameSuccessor {
    me {
      name
      successor {
        successor {
          name
        }
      }
    }
  }
`)(() => {});
const meNameSuccessorEntrypoint = iso(`entrypoint Query.meNameSuccessor`);
export const meNameSuccessorRetainedQuery: RetainedQuery = {
  normalizationAst: wrapResolvedValue(
    meNameSuccessorEntrypoint.networkRequestInfo.normalizationAst,
  ),
  variables: {},
  root: {
    __link: ROOT_ID,
    __typename: 'Query',
  },
};
