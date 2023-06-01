const queryText = 'query header_component {
  current_user {
    avatar_url,
    email,
    name,
  },
}';

type Foo = {poop: String};

module.exports: Foo = { queryText };