// Directives are disallowed when we process the pointer, but allowed at parse time
export const pointer = iso(`
  pointer User.bestFriend to User @directives @allowed @at @parse @time {
    friends {
      id
      closeness
    }
  }
`)();
