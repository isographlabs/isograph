export type Query__PetDetailRoute__raw_response_type = {
  pet____id___v_id?: ({
    id: string,
    age: number,
    best_friend_relationship?: ({
      best_friend: {
        id: string,
        firstName: string,
        lastName: string,
        picture: unknown,
      },
      picture_together?: (unknown | null),
    } | null),
    checkins____skip___l_null____limit___l_null: ReadonlyArray<{
      id: string,
      location: string,
      time: string,
    }>,
    favorite_phrase?: (string | null),
    firstName: string,
    lastName: string,
    nickname?: (string | null),
    potential_new_best_friends: ReadonlyArray<{
      id: string,
      firstName: string,
      lastName: string,
    }>,
    stats?: ({
      cuteness?: (number | null),
      energy?: (number | null),
      hunger?: (number | null),
      intelligence?: (number | null),
      sociability?: (number | null),
      weight?: (number | null),
    } | null),
    tagline: string,
  } | null),
}

