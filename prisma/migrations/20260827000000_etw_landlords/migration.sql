ALTER TABLE "Property" ADD COLUMN "landlordId" TEXT;
ALTER TABLE "Property" ADD COLUMN "accountingMode" TEXT NOT NULL DEFAULT 'standard';

ALTER TABLE "Cost" ADD COLUMN "ownerAmount" REAL;
ALTER TABLE "Cost" ADD COLUMN "tenantAmountOverride" REAL;

CREATE INDEX "Property_landlordId_idx" ON "Property"("landlordId");

-- SQLite cannot add a foreign key constraint with ALTER TABLE in-place.
-- Prisma keeps the relation in the schema for newly created databases.
