pnpm format && \
  cargo clippy && \
  ./scripts/check-git-status.sh
if [ $? -eq 0 ]; then
    echo OK
else
    echo FAIL
fi
