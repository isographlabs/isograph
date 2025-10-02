export default 'mutation MutualBestFriendSetterOtherSide($pet_id: ID!, $new_best_friend_id: ID!) {\
  set_pet_best_friend____id___v_pet_id____new_best_friend_id___v_new_best_friend_id: set_pet_best_friend(id: $pet_id, new_best_friend_id: $new_best_friend_id) {\
    pet {\
      id,\
      best_friend_relationship {\
        best_friend {\
          id,\
          name,\
        },\
      },\
      name,\
    },\
  },\
}';