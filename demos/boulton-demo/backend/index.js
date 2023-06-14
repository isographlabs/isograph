var express = require("express")
const cors = require('cors')

var { graphqlHTTP } = require("express-graphql")
var { buildSchema } = require("graphql")
const { readFileSync } = require("fs")

// Construct a schema, using GraphQL schema language
// You must execute this from the isograph-demo folder...
var schema = buildSchema(readFileSync('./schema.graphql').toString())

// The root provides a resolver function for each API endpoint
var root = {
  current_user: () => userResolver(1),
  current_post: () => postResolver, 
  users: () => [userResolver(1), userResolver(2), userResolver(3)],
  byah: ({foo}) => `byah ${foo}`,
  user: ({id}) => userResolver(id),
};

const names = ["Moe", "Larry", "Curly"];
const avatars = ["https://flxt.tmsimg.com/assets/121814_v9_ba.jpg", "https://flxt.tmsimg.com/assets/189023_v9_ba.jpg", "https://flxt.tmsimg.com/assets/189024_v9_ba.jpg"];
const emails = names.map(name => `${name}@stooges.com`);

const userResolver = (id) => ({
  id: () => id,
  name: () => names[id - 1],
  avatar_url: () => avatars[id - 1],
  email: () => emails[id - 1],
  billing_details: () => billingDetailsResolver
})

const billingDetailsResolver = {
  id: () => 102,
  card_brand: () => 'Visa',
  credit_card_number: () => '1234 5678 9012 3456',
  expiration_date: () => '12/24',
  address: () => '1234 Main St, Anytown, USA',
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