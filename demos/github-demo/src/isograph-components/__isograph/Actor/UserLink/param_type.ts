
export type Actor__UserLink__param = {
  readonly data: {
    /**
A client poiter for the User type.
    */
    readonly asUser: ({
      /**
The Node ID of the User object
      */
      readonly id: string,
      /**
The username used to login.
      */
      readonly login: string,
      /**
The user's Twitter username.
      */
      readonly twitterUsername: (string | null),
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
