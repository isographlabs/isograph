import { type User__RepositoryList__output_type } from '../../User/RepositoryList/output_type';
import { type User____refetch__output_type } from '../../User/__refetch/output_type';

export type Query__HomePageList__param = {
  /**
The currently authenticated user.
  */
  viewer: {
        /**
The username used to login.
    */
login: string,
        /**
The user's public profile name.
    */
name: (string | null),
    RepositoryList: User__RepositoryList__output_type,
    /**
A refetch field for the User type.
    */
    __refetch: User____refetch__output_type,
  },
};
