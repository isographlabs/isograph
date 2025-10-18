export const pointer = iso(`
  pointer User.bestFriend($first: Int!) to User {
    friends(first: $first) {
      id
      closeness
    }
  }
`)();
