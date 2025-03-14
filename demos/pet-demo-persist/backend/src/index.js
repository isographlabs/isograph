import { createServer } from 'node:http';
import { usePersistedOperations } from '@graphql-yoga/plugin-persisted-operations';
import { createYoga } from 'graphql-yoga';
import persistedDocumentsDictionary from '../../src/components/__isograph/persisted_documents.json' with { type: 'json' };
import { schema } from './schema.js';

const persistedDocuments = new Map(
  Object.entries(persistedDocumentsDictionary),
);

// Create a Yoga instance with a GraphQL schema.
const yoga = createYoga({
  schema,
  plugins: [
    usePersistedOperations({
      getPersistedOperation: (hash) => persistedDocuments.get(hash),
    }),
  ],
});

// Pass it into a server to hook into request handlers.
const server = createServer(yoga);

// Start the server and you're done!
server.listen(4000, () => {
  console.info('Server is running on http://localhost:4000/graphql');
});
