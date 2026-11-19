export type Pet__bestFriend__param = {
  readonly data: {
    /**
     * Pets have a very complex social life. Find out more!
     */
    readonly best_friend_relationship: ({
      readonly best_friend: {
        /**
         * The pet's ID
         */
        readonly id: string;
      };
    } | null);
  };
  readonly parameters: Record<PropertyKey, never>;
};
