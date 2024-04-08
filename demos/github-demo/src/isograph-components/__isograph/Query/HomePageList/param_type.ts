import {User__RepositoryList__outputType} from '../../User/RepositoryList/output_type';
import {User____refetch__outputType} from '../../User/__refetch/output_type';

export type Query__HomePageList__param = {
  viewer: {
    login: string,
    name: (string | null),
    RepositoryList: User__RepositoryList__outputType,
    __refetch: User____refetch__outputType,
  },
};
