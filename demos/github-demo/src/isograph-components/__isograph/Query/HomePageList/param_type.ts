import {User__RepositoryList__outputType} from '../../User/RepositoryList/output_type';
import {User____refetch__outputType} from '../../User/__refetch/output_type';

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
    RepositoryList: User__RepositoryList__outputType,
    /**
A refetch field for this object.
    */
    __refetch: User____refetch__outputType,
  },
};
