export const pointer = iso(`
  pointer User.bestFriend to User {
    friends {
      id
      closeness
    }
  }
`)();
