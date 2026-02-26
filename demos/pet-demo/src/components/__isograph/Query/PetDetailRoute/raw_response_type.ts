export type Query__PetDetailRoute__raw_response_type = {
  readonly pet____id___v_id?: ({
    readonly id: string,
    readonly age: number,
    readonly best_friend_relationship?: ({
      readonly best_friend: {
        readonly id: string,
        readonly firstName: string,
        readonly lastName: string,
        readonly picture: unknown,
      },
      readonly picture_together?: (unknown | null),
    } | null),
    readonly checkins____skip___l_null____limit___l_null: ReadonlyArray<{
      readonly id: string,
      readonly location: string,
      readonly time: string,
    }>,
    readonly favorite_phrase?: (string | null),
    readonly firstName: string,
    readonly lastName: string,
    readonly nickname?: (string | null),
    readonly potential_new_best_friends: ReadonlyArray<{
      readonly id: string,
      readonly firstName: string,
      readonly lastName: string,
    }>,
    readonly stats?: ({
      readonly cuteness?: (number | null),
      readonly energy?: (number | null),
      readonly hunger?: (number | null),
      readonly intelligence?: (number | null),
      readonly sociability?: (number | null),
      readonly weight?: (number | null),
    } | null),
    readonly tagline: string,
  } | null),
}

