// Directives are disallowed when we process the pointer, but allowed at parse time
export const pointer = iso(`
  pointer User.bestFriend @directives @allowed @at @parse @time {
    friends {
      id
      closeness
    }
  }
`)();
