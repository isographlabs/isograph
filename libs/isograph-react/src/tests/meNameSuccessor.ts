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
export const meNameSuccessorRetainedQuery = {
  normalizationAst: meNameSuccessorEntrypoint.normalizationAst,
  variables: {},
};
