const path = require('path');

/** @type {import('next').NextConfig} */
const nextConfig = {
  experimental: {
    swcPlugins: [
      [
        path.resolve(__dirname, '../../libs/isograph-swc-plugin/swc_isograph_plugin.wasm'),
        {},
      ],
    ],
  },
};

module.exports = nextConfig;
