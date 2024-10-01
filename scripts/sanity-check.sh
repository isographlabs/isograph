pnpm format && \
  cargo clippy && \
  pnpm build-demos && \
  ./scripts/check-git-status.sh && \
  pnpm -r compile && \
  pnpm -r test
if [ $? -eq 0 ]; then
    echo OK
else
    echo FAIL
fi
