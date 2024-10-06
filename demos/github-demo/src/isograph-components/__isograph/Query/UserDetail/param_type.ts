import { type User__RepositoryList__output_type } from '../../User/RepositoryList/output_type';
import type { Query__UserDetail__parameters } from './parameters_type';

export type Query__UserDetail__param = {
  readonly data: {
    /**
Lookup a user by login.
    */
    readonly user: ({
      /**
The user's public profile name.
      */
      readonly name: (string | null),
      readonly RepositoryList: User__RepositoryList__output_type,
    } | null),
  },
  readonly parameters: Query__UserDetail__parameters,
};
