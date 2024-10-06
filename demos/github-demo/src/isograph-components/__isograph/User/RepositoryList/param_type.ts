import { type User__RepositoryConnection__output_type } from '../../User/RepositoryConnection/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type User__RepositoryList__param = {
  readonly data: {
    readonly RepositoryConnection: LoadableField<{readonly first?: number | null | void, readonly after?: string | null | void}, User__RepositoryConnection__output_type>,
  },
  readonly parameters: Variables,
};
