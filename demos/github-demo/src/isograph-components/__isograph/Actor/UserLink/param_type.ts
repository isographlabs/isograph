
export type Actor__UserLink__param = {
  readonly data: {
    /**
The username of the actor.
    */
    readonly login: string,
    /**
A client pointer for the User type.
    */
    readonly asUser: ({
      /**
The Node ID of the User object
      */
      readonly id: string,
      /**
The user's Twitter username.
      */
      readonly twitterUsername: (string | null),
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
