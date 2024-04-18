import {User__Avatar__outputType} from '../../User/Avatar/output_type';

export type Query__Header__param = {
  /**
The currently authenticated user.
  */
  viewer: {
        /**
The user's public profile name.
    */
name: (string | null),
    Avatar: User__Avatar__outputType,
  },
};
