-- Mark whether a cost category is apportionable to tenants.
ALTER TABLE "CostCategory" ADD COLUMN "apportionable" BOOLEAN NOT NULL DEFAULT true;
