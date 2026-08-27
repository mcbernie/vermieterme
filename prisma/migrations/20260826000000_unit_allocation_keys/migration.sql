-- Add optional per-unit allocation keys for WEG/Betriebskosten keys beyond MEA.
CREATE TABLE "UnitAllocationKey" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "unitId" TEXT NOT NULL,
    "key" TEXT NOT NULL,
    "unitValue" REAL NOT NULL,
    "totalValue" REAL NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" DATETIME NOT NULL,
    CONSTRAINT "UnitAllocationKey_unitId_fkey" FOREIGN KEY ("unitId") REFERENCES "Unit" ("id") ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE UNIQUE INDEX "UnitAllocationKey_unitId_key_key" ON "UnitAllocationKey"("unitId", "key");
CREATE INDEX "UnitAllocationKey_key_idx" ON "UnitAllocationKey"("key");
