-- Initial schema of vermieterme 2.x (SQLite). The MySQL/MariaDB file with the same number
-- must define the same tables, columns, nullability, keys and indexes (checked by a test).
--
-- Portable types (docs/PLAN.md, "Datenbank-Strategie"):
--   ids          TEXT (legacy cuids, new UUID v7)
--   money        INTEGER cents
--   dates        TEXT 'YYYY-MM-DD' (sorts and compares chronologically as text)
--   timestamps   INTEGER Unix milliseconds UTC
--   booleans     INTEGER 0/1
-- Tables are STRICT so SQLite rejects values of the wrong type, like MySQL does, and
-- every text column with a length in MySQL (VARCHAR(n)/CHAR(n)) has the same limit here
-- as CHECK (length(col) <= n); MySQL connections run in strict mode (see pool.rs), so
-- both dialects reject too long values instead of truncating them.
--
-- Every business table carries account_id. References between business tables are
-- composite foreign keys (account_id, x_id) -> parent (account_id, id), so a row can
-- never point at a row of another account.

-- Accounts (customers, "Kunden") and their users ------------------------------------------

CREATE TABLE account (
    id         TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    name       TEXT    NOT NULL CHECK (length(name) <= 255),
    locked     INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE app_user (
    id            TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id    TEXT    NULL REFERENCES account (id) CHECK (length(account_id) <= 36),
    email         TEXT    NOT NULL CHECK (length(email) <= 254),
    name          TEXT    NOT NULL CHECK (length(name) <= 255),
    password_hash TEXT    NULL CHECK (length(password_hash) <= 255),
    role          TEXT    NOT NULL DEFAULT 'member' CHECK (length(role) <= 16),
    locked        INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX app_user_email_key ON app_user (email);
CREATE INDEX app_user_account_idx ON app_user (account_id);

-- External identities (OIDC later); every login path ends in a web session.
CREATE TABLE user_identity (
    provider   TEXT    NOT NULL CHECK (length(provider) <= 64),
    subject    TEXT    NOT NULL CHECK (length(subject) <= 255),
    user_id    TEXT    NOT NULL REFERENCES app_user (id) ON DELETE CASCADE CHECK (length(user_id) <= 36),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (provider, subject)
) STRICT;
CREATE INDEX user_identity_user_idx ON user_identity (user_id);

-- One-time links (purpose 'invite' or 'reset'); only the SHA-256 hash is stored.
CREATE TABLE user_token (
    id         TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    user_id    TEXT    NOT NULL REFERENCES app_user (id) ON DELETE CASCADE CHECK (length(user_id) <= 36),
    purpose    TEXT    NOT NULL CHECK (length(purpose) <= 16),
    token_hash TEXT    NOT NULL CHECK (length(token_hash) <= 64),
    expires_at INTEGER NOT NULL,
    used_at    INTEGER NULL,
    created_at INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX user_token_hash_key ON user_token (token_hash);
CREATE INDEX user_token_user_idx ON user_token (user_id);

-- Web sessions; only the SHA-256 hash of the cookie value is stored.
CREATE TABLE web_session (
    id                TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    token_hash        TEXT    NOT NULL CHECK (length(token_hash) <= 64),
    user_id           TEXT    NOT NULL REFERENCES app_user (id) ON DELETE CASCADE CHECK (length(user_id) <= 36),
    acting_account_id TEXT    NULL REFERENCES account (id) ON DELETE SET NULL CHECK (length(acting_account_id) <= 36),
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
    id         TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    at         INTEGER NOT NULL,
    user_id    TEXT    NULL CHECK (length(user_id) <= 36),
    account_id TEXT    NULL CHECK (length(account_id) <= 36),
    action     TEXT    NOT NULL CHECK (length(action) <= 64),
    entity     TEXT    NOT NULL CHECK (length(entity) <= 64),
    entity_id  TEXT    NULL CHECK (length(entity_id) <= 36)
) STRICT;
CREATE INDEX audit_log_at_idx ON audit_log (at);
CREATE INDEX audit_log_account_at_idx ON audit_log (account_id, at);

-- Settings per account --------------------------------------------------------------------

CREATE TABLE landlord (
    id             TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id     TEXT    NOT NULL REFERENCES account (id) CHECK (length(account_id) <= 36),
    name           TEXT    NOT NULL CHECK (length(name) <= 255),
    street         TEXT    NOT NULL CHECK (length(street) <= 255),
    zip            TEXT    NOT NULL CHECK (length(zip) <= 16),
    city           TEXT    NOT NULL CHECK (length(city) <= 255),
    phone          TEXT    NULL CHECK (length(phone) <= 64),
    email          TEXT    NULL CHECK (length(email) <= 254),
    bank_name      TEXT    NULL CHECK (length(bank_name) <= 255),
    iban           TEXT    NULL CHECK (length(iban) <= 64),
    account_holder TEXT    NULL CHECK (length(account_holder) <= 255),
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX landlord_account_key ON landlord (account_id);

CREATE TABLE pdf_template (
    id         TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id TEXT    NOT NULL REFERENCES account (id) CHECK (length(account_id) <= 36),
    name       TEXT    NOT NULL DEFAULT 'Standard' CHECK (length(name) <= 255),
    config     TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX pdf_template_account_key ON pdf_template (account_id);

-- Master data -----------------------------------------------------------------------------

CREATE TABLE property (
    id           TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id   TEXT    NOT NULL REFERENCES account (id) CHECK (length(account_id) <= 36),
    street       TEXT    NOT NULL CHECK (length(street) <= 255),
    zip          TEXT    NOT NULL CHECK (length(zip) <= 16),
    city         TEXT    NOT NULL CHECK (length(city) <= 255),
    total_shares INTEGER NOT NULL DEFAULT 100,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX property_account_id_key ON property (account_id, id);

-- kind: 'apartment' | 'garage' | 'carport' | 'other' (#2)
CREATE TABLE unit (
    id          TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id  TEXT    NOT NULL CHECK (length(account_id) <= 36),
    property_id TEXT    NOT NULL CHECK (length(property_id) <= 36),
    name        TEXT    NOT NULL CHECK (length(name) <= 255),
    floor       TEXT    NOT NULL CHECK (length(floor) <= 255),
    shares      INTEGER NOT NULL,
    kind        TEXT    NOT NULL DEFAULT 'apartment' CHECK (length(kind) <= 16),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    FOREIGN KEY (account_id, property_id) REFERENCES property (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX unit_account_id_key ON unit (account_id, id);
CREATE INDEX unit_account_property_idx ON unit (account_id, property_id);

-- lease_type: 'standard' | 'index' | 'staffel'; VPI reference value in tenths.
-- app_tokens_revoked_at: tenant-app JWTs issued before this instant are rejected.
CREATE TABLE tenant (
    id                     TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id             TEXT    NOT NULL CHECK (length(account_id) <= 36),
    unit_id                TEXT    NOT NULL CHECK (length(unit_id) <= 36),
    salutation             TEXT    NOT NULL CHECK (length(salutation) <= 64),
    first_name             TEXT    NOT NULL CHECK (length(first_name) <= 255),
    last_name              TEXT    NOT NULL CHECK (length(last_name) <= 255),
    salutation2            TEXT    NULL CHECK (length(salutation2) <= 64),
    first_name2            TEXT    NULL CHECK (length(first_name2) <= 255),
    last_name2             TEXT    NULL CHECK (length(last_name2) <= 255),
    phone                  TEXT    NULL CHECK (length(phone) <= 64),
    email                  TEXT    NULL CHECK (length(email) <= 254),
    bank_name              TEXT    NULL CHECK (length(bank_name) <= 255),
    iban                   TEXT    NULL CHECK (length(iban) <= 64),
    account_holder         TEXT    NULL CHECK (length(account_holder) <= 255),
    move_in_date           TEXT    NOT NULL CHECK (length(move_in_date) <= 10),
    move_out_date          TEXT    NULL CHECK (length(move_out_date) <= 10),
    lease_type             TEXT    NOT NULL DEFAULT 'standard' CHECK (length(lease_type) <= 16),
    index_base_year        INTEGER NULL,
    index_reference_tenths INTEGER NULL,
    index_reference_date   TEXT    NULL CHECK (length(index_reference_date) <= 10),
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
    id         TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id TEXT    NOT NULL CHECK (length(account_id) <= 36),
    tenant_id  TEXT    NOT NULL CHECK (length(tenant_id) <= 36),
    code_hash  TEXT    NOT NULL CHECK (length(code_hash) <= 64),
    expires_at INTEGER NOT NULL,
    used_at    INTEGER NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (account_id, tenant_id) REFERENCES tenant (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX tenant_invite_code_tenant_key ON tenant_invite_code (account_id, tenant_id);
CREATE UNIQUE INDEX tenant_invite_code_hash_key ON tenant_invite_code (code_hash);

-- change_type: 'rent' | 'prepayment'
CREATE TABLE rent_change (
    id             TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id     TEXT    NOT NULL CHECK (length(account_id) <= 36),
    unit_id        TEXT    NOT NULL CHECK (length(unit_id) <= 36),
    change_type    TEXT    NOT NULL CHECK (length(change_type) <= 16),
    amount_cents   INTEGER NOT NULL,
    effective_date TEXT    NOT NULL CHECK (length(effective_date) <= 10),
    reason         TEXT    NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX rent_change_unit_date_idx ON rent_change (account_id, unit_id, effective_date);

-- distribution_key: 'mea' | 'per_notice' | 'per_attachment' | 'per_unit'
CREATE TABLE cost_category (
    id               TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id       TEXT    NOT NULL REFERENCES account (id) CHECK (length(account_id) <= 36),
    name             TEXT    NOT NULL CHECK (length(name) <= 255),
    distribution_key TEXT    NOT NULL CHECK (length(distribution_key) <= 16),
    sort_order       INTEGER NOT NULL DEFAULT 0,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
) STRICT;
CREATE UNIQUE INDEX cost_category_account_id_key ON cost_category (account_id, id);

-- Global consumer price index (public statistics, not per account); value in tenths.
CREATE TABLE vpi_entry (
    id           TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
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
    id             TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id     TEXT    NOT NULL CHECK (length(account_id) <= 36),
    property_id    TEXT    NOT NULL CHECK (length(property_id) <= 36),
    start_date     TEXT    NOT NULL CHECK (length(start_date) <= 10),
    end_date       TEXT    NOT NULL CHECK (length(end_date) <= 10),
    billing_date   TEXT    NULL CHECK (length(billing_date) <= 10),
    sent_date      TEXT    NULL CHECK (length(sent_date) <= 10),
    paid_date      TEXT    NULL CHECK (length(paid_date) <= 10),
    copied_from_id TEXT    NULL REFERENCES billing_period (id) ON DELETE SET NULL CHECK (length(copied_from_id) <= 36),
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
    id                        TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id                TEXT    NOT NULL CHECK (length(account_id) <= 36),
    billing_period_id         TEXT    NOT NULL CHECK (length(billing_period_id) <= 36),
    cost_category_id          TEXT    NOT NULL CHECK (length(cost_category_id) <= 36),
    total_amount_cents        INTEGER NULL,
    reviewed                  INTEGER NOT NULL DEFAULT 1,
    enabled                   INTEGER NOT NULL DEFAULT 1,
    distribution_key_override TEXT    NULL CHECK (length(distribution_key_override) <= 16),
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
    account_id   TEXT    NOT NULL CHECK (length(account_id) <= 36),
    cost_id      TEXT    NOT NULL CHECK (length(cost_id) <= 36),
    unit_id      TEXT    NOT NULL CHECK (length(unit_id) <= 36),
    amount_cents INTEGER NOT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    PRIMARY KEY (account_id, cost_id, unit_id),
    FOREIGN KEY (account_id, cost_id) REFERENCES cost (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, unit_id) REFERENCES unit (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX cost_unit_amount_unit_idx ON cost_unit_amount (account_id, unit_id);

CREATE TABLE prepayment (
    id                   TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id           TEXT    NOT NULL CHECK (length(account_id) <= 36),
    billing_period_id    TEXT    NOT NULL CHECK (length(billing_period_id) <= 36),
    unit_id              TEXT    NOT NULL CHECK (length(unit_id) <= 36),
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
    id                TEXT    NOT NULL PRIMARY KEY CHECK (length(id) <= 36),
    account_id        TEXT    NOT NULL REFERENCES account (id) CHECK (length(account_id) <= 36),
    billing_period_id TEXT    NULL CHECK (length(billing_period_id) <= 36),
    tenant_id         TEXT    NULL CHECK (length(tenant_id) <= 36),
    file_name         TEXT    NOT NULL CHECK (length(file_name) <= 255),
    original_name     TEXT    NOT NULL CHECK (length(original_name) <= 255),
    mime_type         TEXT    NOT NULL CHECK (length(mime_type) <= 127),
    size_bytes        INTEGER NOT NULL,
    category          TEXT    NOT NULL DEFAULT 'other' CHECK (length(category) <= 16),
    created_at        INTEGER NOT NULL,
    FOREIGN KEY (account_id, billing_period_id) REFERENCES billing_period (account_id, id) ON DELETE CASCADE,
    FOREIGN KEY (account_id, tenant_id) REFERENCES tenant (account_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX document_period_idx ON document (account_id, billing_period_id);
CREATE INDEX document_tenant_idx ON document (account_id, tenant_id);
