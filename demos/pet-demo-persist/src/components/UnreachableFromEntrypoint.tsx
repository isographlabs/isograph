import { iso } from '@iso';

/**
 * This file exists just to ensure that we generate the proper artifacts
 * for fields that are not accessible from entrypoints. If this fails to
 * typecheck, we know we have broken something.
 */

export const unreachableFromEntrypoint = iso(`
  field Pet.UnreachableFromEntrypoint {
    id
    Unreachable2
    set_best_friend_do_not_use
  }
`)((data) => {});

export const Unreachable2 = iso(`
  field Pet.Unreachable2 {
    id
  }
`)((data) => {});
