var express = require("express")
const cors = require('cors')

var { graphqlHTTP } = require("express-graphql")
var { buildSchema } = require("graphql")
const { readFileSync } = require("fs")

// Construct a schema, using GraphQL schema language
// You must execute this from the boulton-demo folder...
var schema = buildSchema(readFileSync('./schema.graphql').toString())

// The root provides a resolver function for each API endpoint
var root = {
  hello: () => {
    return "Hello world!"
  },
  current_user: () => userResolver,
  current_post: () => postResolver, 
};

const userResolver = {
  id: () => 1,
  name: () => "John Doe",
  avatar_url: () => 'https://sm.ign.com/ign_ap/cover/a/avatar-gen/avatar-generations_hugw.jpg',
  email: () => 'foo@bar.com',
}

const postResolver = {
  id: () => 2,
  name: () => "My first post",
  content: () => "This is my first post",
  author: () => userResolver,
}

var app = express()
app.use(cors());
app.use(
  "/graphql",
  graphqlHTTP({
    schema: schema,
    rootValue: root,
    graphiql: true,
  })
)
app.listen(4000)
console.log("Running a GraphQL API server at http://localhost:4000/graphql")