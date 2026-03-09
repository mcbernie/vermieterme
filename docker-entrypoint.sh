#!/bin/sh
set -e

# Run database migrations
npx prisma migrate deploy --schema=./prisma/schema.prisma 2>/dev/null || \
  npx prisma db push --schema=./prisma/schema.prisma --accept-data-loss 2>/dev/null || true

exec node server.js
