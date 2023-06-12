export const network = <T>(queryText: string, variables: Object): Promise<T> =>
  fetch("http://localhost:4000/graphql", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
