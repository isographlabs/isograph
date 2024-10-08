import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__RepositoryDetail__output_type } from '../../Query/RepositoryDetail/output_type';
import type { Query__RepositoryPage__parameters } from './parameters_type';

export type Query__RepositoryPage__param = {
  readonly data: {
    readonly Header: Query__Header__output_type,
    readonly RepositoryDetail: Query__RepositoryDetail__output_type,
  },
  readonly parameters: Query__RepositoryPage__parameters,
};
