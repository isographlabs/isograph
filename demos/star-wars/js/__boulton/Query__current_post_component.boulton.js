const queryText = 'query current_post_component {
  current_post {
    content,
    name,
    author {
      avatar_url,
      email,
      name,
    },
  },
}';

type Foo = {poop: String};

module.exports: Foo = { queryText };