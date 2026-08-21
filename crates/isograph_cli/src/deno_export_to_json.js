const path = Deno.args[0];
const href = new URL(path, 'file:///').href;
const m = await import(href);
const config = m.default ?? m;
Deno.stdout.writeSync(new TextEncoder().encode(JSON.stringify(config)));
