#!/usr/bin/env bash

set -euo pipefail

pnpm compile-libs
pnpm test

for folder in github-demo pet-demo vite-demo; do
  (
    cd "./demos/${folder}"
    pnpm tsc
    pnpm lint
  )
done
