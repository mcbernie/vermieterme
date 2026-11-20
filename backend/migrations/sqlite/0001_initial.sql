-- Initial schema of vermieterme 2.x (SQLite). The MySQL/MariaDB file with the same number
-- must define the same tables, columns, nullability, keys and indexes (checked by a test).
--
-- Portable types (docs/PLAN.md, "Datenbank-Strategie"):
--   ids          TEXT (legacy cuids, new UUID v7)
--   money        INTEGER cents
--   dates        TEXT 'YYYY-MM-DD' (sorts and compares chronologically as text)
--   timestamps   INTEGER Unix milliseconds UTC
--   booleans     INTEGER 0/1
-- Tables are STRICT so SQLite rejects values of the wrong type, like MySQL does.
--
-- Every business table carries account_id. References between business tables are
-- composite foreign keys (account_id, x_id) -> parent (account_id, id), so a row can
-- never point at a row of another account.

-- Accounts (customers, "Kunden") and their users ------------------------------------------

CREATE TABLE account (
    id         TEXT    NOT NULL PRIMARY KEY,
    name       TEXT    NOT NULL,
    locked     INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE app_user (
    id            TEXT    NOT NULL PRIMARY KEY,
    account_id    TEXT    NULL REFERENCES account (id),
    email         TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    password_hash TEXT    NULL,
    role          TEXT    NOT NULL DEFAULT 'member',
    locked        INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX app_user_email_key ON app_user (email);
CREATE INDEX app_user_account_idx ON app_user (account_id);

-- External identities (OIDC later); every login path ends in a web session.
CREATE TABLE user_identity (
    provider   TEXT    NOT NULL,
    subject    TEXT    NOT NULL,
    user_id    TEXT    NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (provider, subject)
) STRICT;
CREATE INDEX user_identity_user_idx ON user_identity (user_id);

-- One-time links (purpose 'invite' or 'reset'); only the SHA-256 hash is stored.
CREATE TABLE user_token (
    id         TEXT    NOT NULL PRIMARY KEY,
    user_id    TEXT    NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    purpose    TEXT    NOT NULL,
    token_hash TEXT    NOT NULL,
    expires_at INTEGER NOT NULL,
    used_at    INTEGER NULL,
    created_at INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX user_token_hash_key ON user_token (token_hash);
CREATE INDEX user_token_user_idx ON user_token (user_id);

-- Web sessions; only the SHA-256 hash of the cookie value is stored.
CREATE TABLE web_session (
    id                TEXT    NOT NULL PRIMARY KEY,
    token_hash        TEXT    NOT NULL,
    user_id           TEXT    NOT NULL REFERENCES app_user (id) ON DELETE CASCADE,
    acting_account_id TEXT    NULL REFERENCES account (id) ON DELETE SET NULL,
    created_at        INTEGER NOT NULL,
    last_seen_at      INTEGER NOT NULL,
    expires_at        INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX web_session_token_key ON web_session (token_hash);
CREATE INDEX web_session_user_idx ON web_session (user_id);
CREATE INDEX web_session_acting_idx ON web_session (acting_account_id);

-- Audit log: action, entity and id only, never field values. No foreign keys, so entries
-- survive the deletion of users.
CREATE TABLE audit_log (
    id         TEXT    NOT NULL PRIMARY KEY,
    at         INTEGER NOT NULL,
    user_id    TEXT    NULL,
    account_id TEXT    NULL,
    action     TEXT    NOT NULL,
    entity     TEXT    NOT NULL,
    entity_id  TEXT    NULL
) STRICT;
CREATE INDEX audit_log_at_idx ON audit_log (at);
CREATE INDEX audit_log_account_at_idx ON audit_log (account_id, at);

-- Settings per account --------------------------------------------------------------------

CREATE TABLE landlord (
    id             TEXT    NOT NULL PRIMARY KEY,
    account_id     TEXT    NOT NULL REFERENCES account (id),
    name           TEXT    NOT NULL,
    street         TEXT    NOT NULL,
    zip            TEXT    NOT NULL,
    city           TEXT    NOT NULL,
    phone          TEXT    NULL,
    email          TEXT    NULL,
    bank_name      TEXT    NULL,
    iban           TEXT    NULL,
    account_holder TEXT    NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX landlord_account_key ON landlord (account_id);

CREATE TABLE pdf_template (
    id         TEXT    NOT NULL PRIMARY KEY,
    account_id TEXT    NOT NULL REFERENCES account (id),
    name       TEXT    NOT NULL DEFAULT 'Standard',
    config     TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX pdf_template_account_key ON pdf_template (account_id);

-- Master data -----------------------------------------------------------------------------

CREATE TABLE property (
    id           TEXT    NOT NULL PRIMARY KEY,
    account_id   TEXT    NOT NULL REFERENCES account (id),
    street       TEXT    NOT NULL,
    zip          TEXT    NOT NULL,
    city         TEXT    NOT NULL,
    total_shares INTEGER NOT NULL DEFAULT 100,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX property_account_id_key ON property (account_id, id);

-- kind: 'apartment' | 'garage' | 'carport' | 'other' (#2)
CREATE TABLE unit (
    id          TEXT    NOT NULL PRIMARY KEY,
    account_id  TEXT    NOT NULL,
    property_id TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    floor       TEXT    NOT NULL,
    shares      INTEGER NOT NULL,
    kind        TEXT    NOT NULL DEFAULT 'apartment',
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    FOREIGN KEY (account_id, property_id) REFERENCES property (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX unit_account_id_key ON unit (account_id, id);
CREATE INDEX unit_account_property_idx ON unit (account_id, property_id);

-- lease_type: 'standard' | 'index' | 'staffel'; VPI reference value in tenths.
-- app_tokens_revoked_at: tenant-app JWTs issued before this instant are rejected.
CREATE TABLE tenant (
    id                     TEXT    NOT NULL PRIMARY KEY,
    account_id             TEXT    NOT NULL,
    unit_id                TEXT    NOT NULL,
    salutation             TEXT    NOT NULL,
    first_name             TEXT    NOT NULL,
    last_name              TEXT    NOT NULL,
    salutation2            TEXT    NULL,
    first_name2            TEXT    NULL,
    last_name2             TEXT    NULL,
    phone                  TEXT    NULL,
    email                  TEXT    NULL,
    bank_name              TEXT    NULL,
    iban                   TEXT    NULL,
    account_holder         TEXT    NULL,
    move_in_date           TEXT    NOT NULL,
    move_out_date          TEXT    NULL,
    lease_type             TEXT    NOT NULL DEFAULT 'standard',
    index_base_year        INTEGER NULL,
    index_reference_tenths INTEGER NULL,
    index_reference_date   TEXT    NULL,
    index_min_months       INTEGER NOT NULL DEFAULT 12,
    app_tokens_revoked_at  INTEGER NULL,
    created_at             INTEGER NOT NULL,
    updated_at             INTEGER NOT NULL,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX tenant_account_id_key ON tenant (account_id, id);
CREATE INDEX tenant_account_unit_idx ON tenant (account_id, unit_id);

-- Invitation code of the tenant app: at most one per tenant, stored as HMAC-SHA256.
CREATE TABLE tenant_invite_code (
    id         TEXT    NOT NULL PRIMARY KEY,
    account_id TEXT    NOT NULL,
    tenant_id  TEXT    NOT NULL,
    code_hash  TEXT    NOT NULL,
    expires_at INTEGER NOT NULL,
    used_at    INTEGER NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (account_id, tenant_id) REFERENCES tenant (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX tenant_invite_code_tenant_key ON tenant_invite_code (account_id, tenant_id);
CREATE UNIQUE INDEX tenant_invite_code_hash_key ON tenant_invite_code (code_hash);

-- change_type: 'rent' | 'prepayment'
CREATE TABLE rent_change (
    id             TEXT    NOT NULL PRIMARY KEY,
    account_id     TEXT    NOT NULL,
    unit_id        TEXT    NOT NULL,
    change_type    TEXT    NOT NULL,
    amount_cents   INTEGER NOT NULL,
    effective_date TEXT    NOT NULL,
    reason         TEXT    NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX rent_change_unit_date_idx ON rent_change (account_id, unit_id, effective_date);

-- distribution_key: 'mea' | 'per_notice' | 'per_attachment' | 'per_unit'
CREATE TABLE cost_category (
    id               TEXT    NOT NULL PRIMARY KEY,
    account_id       TEXT    NOT NULL REFERENCES account (id),
    name             TEXT    NOT NULL,
    distribution_key TEXT    NOT NULL,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX cost_category_account_id_key ON cost_category (account_id, id);

-- Global consumer price index (public statistics, not per account); value in tenths.
CREATE TABLE vpi_entry (
    id           TEXT    NOT NULL PRIMARY KEY,
    year         INTEGER NOT NULL,
    month        INTEGER NOT NULL,
    value_tenths INTEGER NOT NULL,
    base_year    INTEGER NOT NULL DEFAULT 2020,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX vpi_entry_period_key ON vpi_entry (year, month, base_year);

-- Billing ---------------------------------------------------------------------------------

CREATE TABLE billing_period (
    id             TEXT    NOT NULL PRIMARY KEY,
    account_id     TEXT    NOT NULL,
    property_id    TEXT    NOT NULL,
    start_date     TEXT    NOT NULL,
    end_date       TEXT    NOT NULL,
    billing_date   TEXT    NULL,
    sent_date      TEXT    NULL,
    paid_date      TEXT    NULL,
    copied_from_id TEXT    NULL REFERENCES billing_period (id) ON DELETE SET NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    FOREIGN KEY (account_id, property_id) REFERENCES property (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX billing_period_account_id_key ON billing_period (account_id, id);
CREATE INDEX billing_period_property_start_idx ON billing_period (account_id, property_id, start_date);
CREATE INDEX billing_period_copied_from_idx ON billing_period (copied_from_id);

-- total_amount_cents NULL = position without amount (#25). version: optimistic lock (#67).
-- Deleting a category with costs is refused (no cascade).
CREATE TABLE cost (
    id                        TEXT    NOT NULL PRIMARY KEY,
    account_id                TEXT    NOT NULL,
    billing_period_id         TEXT    NOT NULL,
    cost_category_id          TEXT    NOT NULL,
    total_amount_cents        INTEGER NULL,
    reviewed                  INTEGER NOT NULL DEFAULT 1,
    enabled                   INTEGER NOT NULL DEFAULT 1,
    distribution_key_override TEXT    NULL,
    version                   INTEGER NOT NULL DEFAULT 1,
    created_at                INTEGER NOT NULL,
    updated_at                INTEGER NOT NULL,
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, cost_category_id) REFERENCES cost_category (account_id, id)
) STRICT;
CREATE UNIQUE INDEX cost_account_id_key ON cost (account_id, id);
CREATE UNIQUE INDEX cost_period_category_key ON cost (account_id, billing_period_id, cost_category_id);
CREATE INDEX cost_category_idx ON cost (account_id, cost_category_id);

-- Amount per unit for 'per_notice' / 'per_attachment' (decision 29, #80). A cleared amount
-- is a deleted row. Concurrency uses the version of the parent cost.
CREATE TABLE cost_unit_amount (
    account_id   TEXT    NOT NULL,
    cost_id      TEXT    NOT NULL,
    unit_id      TEXT    NOT NULL,
    amount_cents INTEGER NOT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    PRIMARY KEY (account_id, cost_id, unit_id),
    FOREIGN KEY (account_id, cost_id) REFERENCES cost (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX cost_unit_amount_unit_idx ON cost_unit_amount (account_id, unit_id);

CREATE TABLE prepayment (
    id                   TEXT    NOT NULL PRIMARY KEY,
    account_id           TEXT    NOT NULL,
    billing_period_id    TEXT    NOT NULL,
    unit_id              TEXT    NOT NULL,
    monthly_amount_cents INTEGER NOT NULL,
    reviewed             INTEGER NOT NULL DEFAULT 1,
    version              INTEGER NOT NULL DEFAULT 1,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX prepayment_period_unit_key ON prepayment (account_id, billing_period_id, unit_id);
CREATE INDEX prepayment_unit_idx ON prepayment (account_id, unit_id);

-- Documents ---------------------------------------------------------------------------------

-- file_name: stored file below UPLOAD_DIR/{account_id}/; category: 'invoice' | 'receipt' |
-- 'contract' | 'other'. Without billing period and tenant the document is visible only on
-- the web (decision 20).
CREATE TABLE document (
    id                TEXT    NOT NULL PRIMARY KEY,
    account_id        TEXT    NOT NULL REFERENCES account (id),
    billing_period_id TEXT    NULL,
    tenant_id         TEXT    NULL,
    file_name         TEXT    NOT NULL,
    original_name     TEXT    NOT NULL,
    mime_type         TEXT    NOT NULL,
    size_bytes        INTEGER NOT NULL,
    category          TEXT    NOT NULL DEFAULT 'other',
    created_at        INTEGER NOT NULL,
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, tenant_id) REFERENCES tenant (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX document_period_idx ON document (account_id, billing_period_id);
CREATE INDEX document_tenant_idx ON document (account_id, tenant_id);
