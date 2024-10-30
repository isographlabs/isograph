
export type PullRequest__PullRequestLink__param = {
  readonly data: {
    /**
Identifies the pull request number.
    */
    readonly number: number,
    /**
The repository associated with this node.
    */
    readonly repository: {
      /**
The name of the repository.
      */
      readonly name: string,
      /**
The User owner of the repository.
      */
      readonly owner: {
        /**
The username used to login.
        */
        readonly login: string,
        /**
A client poiter for the User type.
        */
        readonly asUser: ({
          /**
The Node ID of the User object
          */
          readonly id: string,
          /**
The user's public profile bio.
          */
          readonly bio: (string | null),
        } | null),
      },
    },
  },
  readonly parameters: Record<PropertyKey, never>,
};
