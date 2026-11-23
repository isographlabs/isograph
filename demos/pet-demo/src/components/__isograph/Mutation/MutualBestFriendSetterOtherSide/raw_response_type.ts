export type Mutation__MutualBestFriendSetterOtherSide__raw_response_type = {
  readonly set_pet_best_friend____id___v_pet_id____new_best_friend_id___v_new_best_friend_id: {
    readonly pet: {
      readonly id: string,
      readonly best_friend_relationship?: ({
        readonly best_friend: {
          readonly id: string,
          readonly firstName: string,
          readonly lastName: string,
        },
      } | null),
      readonly firstName: string,
      readonly lastName: string,
    },
  },
}

