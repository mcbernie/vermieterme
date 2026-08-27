-- Mark cost categories that require an external statement or attachment.
ALTER TABLE "CostCategory" ADD COLUMN "requiresAttachment" BOOLEAN NOT NULL DEFAULT false;
