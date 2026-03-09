#!/bin/sh
set -e

echo "Running database setup..."
node node_modules/prisma/build/index.js db push --schema=./prisma/schema.prisma --skip-generate
echo "Database ready."

exec node server.js
