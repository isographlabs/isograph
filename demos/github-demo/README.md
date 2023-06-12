# github demo

## Running locally

In order to run this demo locally:

- create an .env.local file in this folder. It will be ignored by git. It's contents should be:

```sh
NEXT_PUBLIC_GITHUB_TOKEN=$SOME_TOKEN
```

Where `$SOME_TOKEN` is a personal access token. It only needs read access.

- Then, `cargo build` from the root of the repo.
- Then, `yarn && npm run dev`.
- Then, navigate to `localhost:3000`.
