const path = process.argv[process.argv.length - 1];
import('node:url')
  .then(({ pathToFileURL }) => import(pathToFileURL(path).href))
  .then((m) => {
    const config = m.default ?? m;
    process.stdout.write(JSON.stringify(config));
  })
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
