export const pointer = iso(`
  pointer User.bestFriend {
    friends {
      id
      closeness
    }
  }
`)();
