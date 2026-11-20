-- Initial schema of vermieterme 2.x (MySQL 8 / MariaDB 10.11). The SQLite file with the
-- same number must define the same tables, columns, nullability, keys and indexes
-- (checked by a test).
--
-- Portable types (docs/PLAN.md, "Datenbank-Strategie"), chosen for the sqlx `Any` driver:
--   ids          VARCHAR(36) (legacy cuids, new UUID v7)
--   money        BIGINT cents
--   dates        CHAR(10) 'YYYY-MM-DD': DATE is not decodable through `Any`, and the text
--                form sorts and compares chronologically like in SQLite
--   timestamps   BIGINT Unix milliseconds UTC (no DATETIME/time zone handling)
--   booleans     SMALLINT 0/1: TINYINT/BOOLEAN is not decodable through `Any`
--   integers     BIGINT (all integers decode as i64)
-- utf8mb4_bin keeps comparisons and unique keys byte-exact like SQLite (e-mail addresses
-- are normalised to lower case in the application), except for trailing spaces: it
-- compares with PAD SPACE ('a' = 'a  '), and spaces beyond a VARCHAR limit are cut with a
-- note only, even in strict mode. The API therefore trims every text input (P3/P4).
-- Free text: TEXT holds at most 65 535 bytes, so pdf_template.config is limited to
-- 16 000 characters (4 bytes each in utf8mb4) by a CHECK, like in SQLite.
--
-- Foreign keys are table constraints (InnoDB ignores inline REFERENCES), and every
-- foreign key has an explicitly named index, so MySQL creates no implicit ones.
-- Every business table carries account_id; references between business tables are
-- composite (account_id, x_id) -> parent (account_id, id). Only exception: the
-- informational billing_period.copied_from_id (see there).

-- Accounts (customers, "Kunden") and their users ------------------------------------------

CREATE TABLE account (
    id         VARCHAR(36)  NOT NULL,
    name       VARCHAR(255) NOT NULL,
    locked     SMALLINT     NOT NULL DEFAULT 0,
    created_at BIGINT       NOT NULL,
    updated_at BIGINT       NOT NULL,
    PRIMARY KEY (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- A member always belongs to an account; only an admin may have none (bootstrap).
-- Legacy import (P9): Prisma User.name and User.email are nullable, app_user.name and
-- app_user.email are not, so the import needs a replacement rule for missing values.
CREATE TABLE app_user (
    id            VARCHAR(36)  NOT NULL,
    account_id    VARCHAR(36)  NULL,
    email         VARCHAR(254) NOT NULL,
    name          VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NULL,
    role          VARCHAR(16)  NOT NULL DEFAULT 'member',
    locked        SMALLINT     NOT NULL DEFAULT 0,
    created_at    BIGINT       NOT NULL,
    updated_at    BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY app_user_email_key (email),
    KEY app_user_account_idx (account_id),
    FOREIGN KEY (account_id) REFERENCES account (id),
    CONSTRAINT app_user_member_account_chk CHECK (role = 'admin' OR account_id IS NOT NULL)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- External identities (OIDC later); every login path ends in a web session.
CREATE TABLE user_identity (
    provider   VARCHAR(64)  NOT NULL,
    subject    VARCHAR(255) NOT NULL,
    user_id    VARCHAR(36)  NOT NULL,
    created_at BIGINT       NOT NULL,
    PRIMARY KEY (provider, subject),
    KEY user_identity_user_idx (user_id),
    FOREIGN KEY (user_id) REFERENCES app_user (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- One-time links (purpose 'invite' or 'reset'); only the SHA-256 hash is stored.
CREATE TABLE user_token (
    id         VARCHAR(36) NOT NULL,
    user_id    VARCHAR(36) NOT NULL,
    purpose    VARCHAR(16) NOT NULL,
    token_hash CHAR(64)    NOT NULL,
    expires_at BIGINT      NOT NULL,
    used_at    BIGINT      NULL,
    created_at BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY user_token_hash_key (token_hash),
    KEY user_token_user_idx (user_id),
    FOREIGN KEY (user_id) REFERENCES app_user (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Web sessions; only the SHA-256 hash of the cookie value is stored. Housekeeping
-- deletes by expires_at OR last_seen_at (one index per condition).
CREATE TABLE web_session (
    id                VARCHAR(36) NOT NULL,
    token_hash        CHAR(64)    NOT NULL,
    user_id           VARCHAR(36) NOT NULL,
    acting_account_id VARCHAR(36) NULL,
    created_at        BIGINT      NOT NULL,
    last_seen_at      BIGINT      NOT NULL,
    expires_at        BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY web_session_token_key (token_hash),
    KEY web_session_user_idx (user_id),
    KEY web_session_acting_idx (acting_account_id),
    KEY web_session_expires_idx (expires_at),
    KEY web_session_last_seen_idx (last_seen_at),
    FOREIGN KEY (user_id) REFERENCES app_user (id) ON DELETE CASCADE,
    FOREIGN KEY (acting_account_id) REFERENCES account (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Audit log: action, entity and id only, never field values. No foreign keys, so entries
-- survive the deletion of users. Paging uses the cursor (at, id) in the order
-- at DESC, id DESC (both indexes end in id).
CREATE TABLE audit_log (
    id         VARCHAR(36) NOT NULL,
    at         BIGINT      NOT NULL,
    user_id    VARCHAR(36) NULL,
    account_id VARCHAR(36) NULL,
    action     VARCHAR(64) NOT NULL,
    entity     VARCHAR(64) NOT NULL,
    entity_id  VARCHAR(36) NULL,
    PRIMARY KEY (id),
    KEY audit_log_at_idx (at, id),
    KEY audit_log_account_at_idx (account_id, at, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Settings per account --------------------------------------------------------------------
-- Legacy import (P9): LandlordInfo and PdfTemplate may hold several rows in Prisma; here
-- they are unique per account, so the import must pick one row per account.

CREATE TABLE landlord (
    id             VARCHAR(36)  NOT NULL,
    account_id     VARCHAR(36)  NOT NULL,
    name           VARCHAR(255) NOT NULL,
    street         VARCHAR(255) NOT NULL,
    zip            VARCHAR(16)  NOT NULL,
    city           VARCHAR(255) NOT NULL,
    phone          VARCHAR(64)  NULL,
    email          VARCHAR(254) NULL,
    bank_name      VARCHAR(255) NULL,
    iban           VARCHAR(64)  NULL,
    account_holder VARCHAR(255) NULL,
    created_at     BIGINT       NOT NULL,
    updated_at     BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY landlord_account_key (account_id),
    FOREIGN KEY (account_id) REFERENCES account (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE pdf_template (
    id         VARCHAR(36)  NOT NULL,
    account_id VARCHAR(36)  NOT NULL,
    name       VARCHAR(255) NOT NULL DEFAULT 'Standard',
    config     TEXT         NOT NULL,
    created_at BIGINT       NOT NULL,
    updated_at BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY pdf_template_account_key (account_id),
    FOREIGN KEY (account_id) REFERENCES account (id),
    CONSTRAINT pdf_template_config_len_chk CHECK (CHAR_LENGTH(config) <= 16000)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Master data -----------------------------------------------------------------------------

CREATE TABLE property (
    id           VARCHAR(36)  NOT NULL,
    account_id   VARCHAR(36)  NOT NULL,
    street       VARCHAR(255) NOT NULL,
    zip          VARCHAR(16)  NOT NULL,
    city         VARCHAR(255) NOT NULL,
    total_shares BIGINT       NOT NULL DEFAULT 100,
    created_at   BIGINT       NOT NULL,
    updated_at   BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY property_account_id_key (account_id, id),
    FOREIGN KEY (account_id) REFERENCES account (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- kind: 'apartment' | 'garage' | 'carport' | 'other' (#2)
CREATE TABLE unit (
    id          VARCHAR(36)  NOT NULL,
    account_id  VARCHAR(36)  NOT NULL,
    property_id VARCHAR(36)  NOT NULL,
    name        VARCHAR(255) NOT NULL,
    floor       VARCHAR(255) NOT NULL,
    shares      BIGINT       NOT NULL,
    kind        VARCHAR(16)  NOT NULL DEFAULT 'apartment',
    created_at  BIGINT       NOT NULL,
    updated_at  BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY unit_account_id_key (account_id, id),
    KEY unit_account_property_idx (account_id, property_id),
    FOREIGN KEY (account_id, property_id) REFERENCES property (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- lease_type: 'standard' | 'index' | 'staffel'; VPI reference value in tenths.
-- app_tokens_revoked_at: tenant-app JWTs issued before this instant are rejected.
CREATE TABLE tenant (
    id                     VARCHAR(36)  NOT NULL,
    account_id             VARCHAR(36)  NOT NULL,
    unit_id                VARCHAR(36)  NOT NULL,
    salutation             VARCHAR(64)  NOT NULL,
    first_name             VARCHAR(255) NOT NULL,
    last_name              VARCHAR(255) NOT NULL,
    salutation2            VARCHAR(64)  NULL,
    first_name2            VARCHAR(255) NULL,
    last_name2             VARCHAR(255) NULL,
    phone                  VARCHAR(64)  NULL,
    email                  VARCHAR(254) NULL,
    bank_name              VARCHAR(255) NULL,
    iban                   VARCHAR(64)  NULL,
    account_holder         VARCHAR(255) NULL,
    move_in_date           CHAR(10)     NOT NULL,
    move_out_date          CHAR(10)     NULL,
    lease_type             VARCHAR(16)  NOT NULL DEFAULT 'standard',
    index_base_year        BIGINT       NULL,
    index_reference_tenths BIGINT       NULL,
    index_reference_date   CHAR(10)     NULL,
    index_min_months       BIGINT       NOT NULL DEFAULT 12,
    app_tokens_revoked_at  BIGINT       NULL,
    created_at             BIGINT       NOT NULL,
    updated_at             BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY tenant_account_id_key (account_id, id),
    KEY tenant_account_unit_idx (account_id, unit_id),
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Invitation code of the tenant app: at most one per tenant, stored as HMAC-SHA256.
-- code_hash is unique across all accounts; with 6 characters collisions are possible, so
-- the code generator (P7) must retry with a new code on Conflict.
CREATE TABLE tenant_invite_code (
    id         VARCHAR(36) NOT NULL,
    account_id VARCHAR(36) NOT NULL,
    tenant_id  VARCHAR(36) NOT NULL,
    code_hash  CHAR(64)    NOT NULL,
    expires_at BIGINT      NOT NULL,
    used_at    BIGINT      NULL,
    created_at BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY tenant_invite_code_tenant_key (account_id, tenant_id),
    UNIQUE KEY tenant_invite_code_hash_key (code_hash),
    FOREIGN KEY (account_id, tenant_id) REFERENCES tenant (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- change_type: 'rent' | 'prepayment'
CREATE TABLE rent_change (
    id             VARCHAR(36) NOT NULL,
    account_id     VARCHAR(36) NOT NULL,
    unit_id        VARCHAR(36) NOT NULL,
    change_type    VARCHAR(16) NOT NULL,
    amount_cents   BIGINT      NOT NULL,
    effective_date CHAR(10)    NOT NULL,
    reason         VARCHAR(2000) NULL,
    created_at     BIGINT      NOT NULL,
    updated_at     BIGINT      NOT NULL,
    PRIMARY KEY (id),
    KEY rent_change_unit_date_idx (account_id, unit_id, effective_date),
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- distribution_key: 'mea' | 'per_notice' | 'per_attachment' | 'per_unit'
CREATE TABLE cost_category (
    id               VARCHAR(36)  NOT NULL,
    account_id       VARCHAR(36)  NOT NULL,
    name             VARCHAR(255) NOT NULL,
    distribution_key VARCHAR(16)  NOT NULL,
    sort_order       BIGINT       NOT NULL DEFAULT 0,
    created_at       BIGINT       NOT NULL,
    updated_at       BIGINT       NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY cost_category_account_id_key (account_id, id),
    FOREIGN KEY (account_id) REFERENCES account (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Global consumer price index (public statistics, not per account); value in tenths.
CREATE TABLE vpi_entry (
    id           VARCHAR(36) NOT NULL,
    year         BIGINT      NOT NULL,
    month        BIGINT      NOT NULL,
    value_tenths BIGINT      NOT NULL,
    base_year    BIGINT      NOT NULL DEFAULT 2020,
    created_at   BIGINT      NOT NULL,
    updated_at   BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY vpi_entry_period_key (year, month, base_year)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Billing ---------------------------------------------------------------------------------

-- copied_from_id is informational only (the period a copy was made from). Its foreign key
-- has one column because ON DELETE SET NULL cannot null a composite key without also
-- nulling account_id; it therefore does not stop a reference into another account. The
-- copy function must load the source period through the account scope and may only set
-- copied_from_id to a period of the same account; nothing may read across this link.
CREATE TABLE billing_period (
    id             VARCHAR(36) NOT NULL,
    account_id     VARCHAR(36) NOT NULL,
    property_id    VARCHAR(36) NOT NULL,
    start_date     CHAR(10)    NOT NULL,
    end_date       CHAR(10)    NOT NULL,
    billing_date   CHAR(10)    NULL,
    sent_date      CHAR(10)    NULL,
    paid_date      CHAR(10)    NULL,
    copied_from_id VARCHAR(36) NULL,
    created_at     BIGINT      NOT NULL,
    updated_at     BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY billing_period_account_id_key (account_id, id),
    KEY billing_period_property_start_idx (account_id, property_id, start_date),
    KEY billing_period_copied_from_idx (copied_from_id),
    FOREIGN KEY (account_id, property_id) REFERENCES property (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (copied_from_id) REFERENCES billing_period (id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Legacy import (P9): Prisma Cost and Prepayment have no updatedAt; the import sets
-- updated_at to created_at.
-- total_amount_cents NULL = position without amount (#25). version: optimistic lock (#67).
-- Deleting a category with costs is refused (no cascade).
CREATE TABLE cost (
    id                        VARCHAR(36) NOT NULL,
    account_id                VARCHAR(36) NOT NULL,
    billing_period_id         VARCHAR(36) NOT NULL,
    cost_category_id          VARCHAR(36) NOT NULL,
    total_amount_cents        BIGINT      NULL,
    reviewed                  SMALLINT    NOT NULL DEFAULT 1,
    enabled                   SMALLINT    NOT NULL DEFAULT 1,
    distribution_key_override VARCHAR(16) NULL,
    version                   BIGINT      NOT NULL DEFAULT 1,
    created_at                BIGINT      NOT NULL,
    updated_at                BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY cost_account_id_key (account_id, id),
    UNIQUE KEY cost_period_category_key (account_id, billing_period_id, cost_category_id),
    KEY cost_category_idx (account_id, cost_category_id),
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, cost_category_id) REFERENCES cost_category (account_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Amount per unit for 'per_notice' / 'per_attachment' (decision 29, #80). A cleared amount
-- is a deleted row. Concurrency uses the version of the parent cost.
CREATE TABLE cost_unit_amount (
    account_id   VARCHAR(36) NOT NULL,
    cost_id      VARCHAR(36) NOT NULL,
    unit_id      VARCHAR(36) NOT NULL,
    amount_cents BIGINT      NOT NULL,
    created_at   BIGINT      NOT NULL,
    updated_at   BIGINT      NOT NULL,
    PRIMARY KEY (account_id, cost_id, unit_id),
    KEY cost_unit_amount_unit_idx (account_id, unit_id),
    FOREIGN KEY (account_id, cost_id) REFERENCES cost (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE prepayment (
    id                   VARCHAR(36) NOT NULL,
    account_id           VARCHAR(36) NOT NULL,
    billing_period_id    VARCHAR(36) NOT NULL,
    unit_id              VARCHAR(36) NOT NULL,
    monthly_amount_cents BIGINT      NOT NULL,
    reviewed             SMALLINT    NOT NULL DEFAULT 1,
    version              BIGINT      NOT NULL DEFAULT 1,
    created_at           BIGINT      NOT NULL,
    updated_at           BIGINT      NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY prepayment_period_unit_key (account_id, billing_period_id, unit_id),
    KEY prepayment_unit_idx (account_id, unit_id),
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

-- Documents ---------------------------------------------------------------------------------

-- file_name: stored file below UPLOAD_DIR/{account_id}/; category: 'invoice' | 'receipt' |
-- 'contract' | 'other'. Without billing period and tenant the document is visible only on
-- the web (decision 20).
CREATE TABLE document (
    id                VARCHAR(36)  NOT NULL,
    account_id        VARCHAR(36)  NOT NULL,
    billing_period_id VARCHAR(36)  NULL,
    tenant_id         VARCHAR(36)  NULL,
    file_name         VARCHAR(255) NOT NULL,
    original_name     VARCHAR(255) NOT NULL,
    mime_type         VARCHAR(127) NOT NULL,
    size_bytes        BIGINT       NOT NULL,
    category          VARCHAR(16)  NOT NULL DEFAULT 'other',
    created_at        BIGINT       NOT NULL,
    PRIMARY KEY (id),
    KEY document_period_idx (account_id, billing_period_id),
    KEY document_tenant_idx (account_id, tenant_id),
    FOREIGN KEY (account_id) REFERENCES account (id),
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, tenant_id) REFERENCES tenant (account_id, id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
