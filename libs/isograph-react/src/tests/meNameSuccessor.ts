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
import meNameSuccessorEntrypoint from './__isograph/Query/meNameSuccessor/entrypoint';
iso(`entrypoint Query.meNameSuccessor`);
export const meNameSuccessorRetainedQuery = {
  normalizationAst: meNameSuccessorEntrypoint.normalizationAst,
  variables: {},
};
