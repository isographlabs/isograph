const queryText = 'query root {
  current_post {
    content,
    name,
    author {
      avatar_url,
      email,
      name,
    },
  },
  current_user {
    avatar_url,
    email,
    name,
  },
}';

type Foo = {poop: String};

module.exports: Foo = { queryText };