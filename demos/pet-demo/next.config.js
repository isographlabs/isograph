const path = require('path');

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  experimental: {
    swcPlugins: [
      [
        path.resolve(
          __dirname,
          '../../libs/isograph-swc-plugin/swc_isograph_plugin.wasm',
        ),
        {
          // must be an absolute path
          root_dir: path.resolve(__dirname, '.'),
          config: require('./isograph.config.json'),
        },
      ],
    ],
  },
};

module.exports = nextConfig;
