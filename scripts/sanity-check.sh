pnpm format && \
  cargo clippy && \
  pnpm build-demos && \
  ./scripts/check-git-status.sh && \
  pnpm compile-libs && \
  pnpm test
if [ $? -eq 0 ]; then
    echo OK
else
    echo FAIL
fi
