import { type User__Avatar__output_type } from '../../User/Avatar/output_type';

export type Query__Header__param = {
  /**
The currently authenticated user.
  */
  viewer: {
        /**
The user's public profile name.
    */
name: (string | null),
    Avatar: User__Avatar__output_type,
  },
};
