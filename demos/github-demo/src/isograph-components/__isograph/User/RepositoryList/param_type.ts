import { type User__RepositoryConnection__output_type } from '../../User/RepositoryConnection/output_type';
import { type LoadableField } from '@isograph/react';
import { type User__RepositoryConnection__param } from '../../User/RepositoryConnection/param_type';

export type User__RepositoryList__param = {
  readonly data: {
    readonly firstPage: User__RepositoryConnection__output_type,
    readonly RepositoryConnection: LoadableField<
      User__RepositoryConnection__param,
      User__RepositoryConnection__output_type
    >,
  },
  readonly parameters: Record<string, never>,
};
