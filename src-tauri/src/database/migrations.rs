use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // Migration 001: Core schema — all tables
    r#"
    -- ============================================================
    -- APP SETTINGS (key-value)
    -- ============================================================
    CREATE TABLE IF NOT EXISTS app_settings (
        key         TEXT PRIMARY KEY,
        value       TEXT,
        updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- ============================================================
    -- ADMIN USER (single admin only)
    -- ============================================================
    CREATE TABLE IF NOT EXISTS admin_users (
        id              INTEGER PRIMARY KEY,
        username        TEXT NOT NULL UNIQUE,
        password_hash   TEXT NOT NULL,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- ============================================================
    -- COMPANY
    -- ============================================================
    CREATE TABLE IF NOT EXISTS companies (
        id              INTEGER PRIMARY KEY,
        name            TEXT NOT NULL,
        legal_name      TEXT,
        address         TEXT,
        phone           TEXT,
        email           TEXT,
        tax_id          TEXT,
        currency        TEXT NOT NULL DEFAULT 'USD',
        logo_path       TEXT,
        is_active       INTEGER NOT NULL DEFAULT 1,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- ============================================================
    -- DEPARTMENTS
    -- ============================================================
    CREATE TABLE IF NOT EXISTS departments (
        id              INTEGER PRIMARY KEY,
        company_id      INTEGER NOT NULL,
        name            TEXT NOT NULL,
        code            TEXT,
        description     TEXT,
        is_active       INTEGER NOT NULL DEFAULT 1,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (company_id) REFERENCES companies(id)
    );
    CREATE INDEX IF NOT EXISTS idx_departments_company ON departments(company_id);

    -- ============================================================
    -- POSITIONS
    -- ============================================================
    CREATE TABLE IF NOT EXISTS positions (
        id              INTEGER PRIMARY KEY,
        department_id   INTEGER NOT NULL,
        title           TEXT NOT NULL,
        code            TEXT,
        description     TEXT,
        base_salary     REAL,
        is_active       INTEGER NOT NULL DEFAULT 1,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (department_id) REFERENCES departments(id)
    );
    CREATE INDEX IF NOT EXISTS idx_positions_department ON positions(department_id);

    -- ============================================================
    -- EMPLOYEES
    -- ============================================================
    CREATE TABLE IF NOT EXISTS employees (
        id                  INTEGER PRIMARY KEY,
        employee_code       TEXT NOT NULL UNIQUE,
        first_name          TEXT NOT NULL,
        last_name           TEXT NOT NULL,
        gender              TEXT,
        date_of_birth       TEXT,
        national_id         TEXT,
        phone               TEXT,
        email               TEXT,
        address             TEXT,
        department_id       INTEGER,
        position_id         INTEGER,
        hire_date           TEXT,
        termination_date    TEXT,
        employment_status   TEXT NOT NULL DEFAULT 'active',
        bank_account        TEXT,
        bank_name           TEXT,
        is_active           INTEGER NOT NULL DEFAULT 1,
        created_at          TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at          TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (department_id) REFERENCES departments(id),
        FOREIGN KEY (position_id) REFERENCES positions(id)
    );
    CREATE INDEX IF NOT EXISTS idx_employees_department ON employees(department_id);
    CREATE INDEX IF NOT EXISTS idx_employees_position ON employees(position_id);
    CREATE INDEX IF NOT EXISTS idx_employees_status ON employees(employment_status);

    -- ============================================================
    -- EMPLOYEE SALARY HISTORY
    -- ============================================================
    CREATE TABLE IF NOT EXISTS employee_salary_history (
        id              INTEGER PRIMARY KEY,
        employee_id     INTEGER NOT NULL,
        base_salary     REAL NOT NULL,
        effective_date  TEXT NOT NULL,
        reason          TEXT,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (employee_id) REFERENCES employees(id)
    );
    CREATE INDEX IF NOT EXISTS idx_salary_history_employee ON employee_salary_history(employee_id);

    -- ============================================================
    -- SALARY COMPONENTS (configurable: allowance, deduction, tax)
    -- calc_type: 'percentage' | 'fixed' | 'formula'
    -- component_type: 'earning' | 'deduction' | 'tax' | 'contribution'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS salary_components (
        id                  INTEGER PRIMARY KEY,
        name                TEXT NOT NULL,
        code                TEXT NOT NULL UNIQUE,
        component_type      TEXT NOT NULL,
        calc_type           TEXT NOT NULL,
        default_value       REAL,
        formula_expression  TEXT,
        applies_to          TEXT NOT NULL DEFAULT 'all',
        is_taxable          INTEGER NOT NULL DEFAULT 0,
        is_pensionable      INTEGER NOT NULL DEFAULT 0,
        is_active           INTEGER NOT NULL DEFAULT 1,
        created_at          TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_salary_components_type ON salary_components(component_type);
    CREATE INDEX IF NOT EXISTS idx_salary_components_active ON salary_components(is_active);

    -- ============================================================
    -- EMPLOYEE COMPONENTS (component overrides per employee)
    -- ============================================================
    CREATE TABLE IF NOT EXISTS employee_components (
        id              INTEGER PRIMARY KEY,
        employee_id     INTEGER NOT NULL,
        component_id    INTEGER NOT NULL,
        override_value  REAL,
        is_active       INTEGER NOT NULL DEFAULT 1,
        effective_date  TEXT NOT NULL DEFAULT (datetime('now')),
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (employee_id) REFERENCES employees(id),
        FOREIGN KEY (component_id) REFERENCES salary_components(id)
    );
    CREATE INDEX IF NOT EXISTS idx_employee_components_employee ON employee_components(employee_id);
    CREATE INDEX IF NOT EXISTS idx_employee_components_component ON employee_components(component_id);

    -- ============================================================
    -- PAYROLL CONFIGURATION (formula versions, tax rules, thresholds)
    -- Stores versioned rule sets so historical payroll is frozen.
    -- ============================================================
    CREATE TABLE IF NOT EXISTS payroll_configurations (
        id              INTEGER PRIMARY KEY,
        name            TEXT NOT NULL,
        config_key      TEXT NOT NULL,
        config_value    TEXT NOT NULL,
        version         INTEGER NOT NULL DEFAULT 1,
        is_active       INTEGER NOT NULL DEFAULT 1,
        effective_date  TEXT NOT NULL DEFAULT (datetime('now')),
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE (config_key, version)
    );
    CREATE INDEX IF NOT EXISTS idx_payroll_config_key ON payroll_configurations(config_key);
    CREATE INDEX IF NOT EXISTS idx_payroll_config_active ON payroll_configurations(is_active);

    -- ============================================================
    -- PAYROLL PERIODS
    -- status: 'open' | 'processing' | 'closed' | 'locked'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS payroll_periods (
        id              INTEGER PRIMARY KEY,
        period_name     TEXT NOT NULL,
        start_date      TEXT NOT NULL,
        end_date        TEXT NOT NULL,
        pay_date        TEXT,
        status          TEXT NOT NULL DEFAULT 'open',
        config_version  INTEGER NOT NULL DEFAULT 1,
        closed_at       TEXT,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_payroll_periods_status ON payroll_periods(status);

    -- ============================================================
    -- PAYROLL RECORDS (per employee per period)
    -- calculation_snapshot stores full JSON of all inputs/outputs
    -- so historical records never need recalculation.
    -- ============================================================
    CREATE TABLE IF NOT EXISTS payroll_records (
        id                      INTEGER PRIMARY KEY,
        period_id               INTEGER NOT NULL,
        employee_id             INTEGER NOT NULL,
        base_salary             REAL NOT NULL,
        gross_earnings          REAL NOT NULL DEFAULT 0,
        total_deductions        REAL NOT NULL DEFAULT 0,
        total_tax               REAL NOT NULL DEFAULT 0,
        net_pay                 REAL NOT NULL DEFAULT 0,
        calculation_snapshot    TEXT,
        status                  TEXT NOT NULL DEFAULT 'calculated',
        config_version          INTEGER NOT NULL,
        created_at              TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at              TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (period_id) REFERENCES payroll_periods(id),
        FOREIGN KEY (employee_id) REFERENCES employees(id)
    );
    CREATE INDEX IF NOT EXISTS idx_payroll_records_period ON payroll_records(period_id);
    CREATE INDEX IF NOT EXISTS idx_payroll_records_employee ON payroll_records(employee_id);

    -- ============================================================
    -- PAYROLL ITEMS (individual line items — frozen snapshot)
    -- ============================================================
    CREATE TABLE IF NOT EXISTS payroll_items (
        id                  INTEGER PRIMARY KEY,
        payroll_record_id   INTEGER NOT NULL,
        component_id        INTEGER,
        component_name      TEXT NOT NULL,
        component_code      TEXT NOT NULL,
        component_type      TEXT NOT NULL,
        calc_type           TEXT NOT NULL,
        rate_or_value       REAL,
        formula_expression  TEXT,
        amount              REAL NOT NULL,
        created_at          TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (payroll_record_id) REFERENCES payroll_records(id),
        FOREIGN KEY (component_id) REFERENCES salary_components(id)
    );
    CREATE INDEX IF NOT EXISTS idx_payroll_items_record ON payroll_items(payroll_record_id);

    -- ============================================================
    -- LOANS
    -- status: 'active' | 'paid' | 'cancelled'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS loans (
        id              INTEGER PRIMARY KEY,
        employee_id     INTEGER NOT NULL,
        principal       REAL NOT NULL,
        interest_rate   REAL NOT NULL DEFAULT 0,
        total_amount    REAL NOT NULL,
        installment_amount REAL NOT NULL,
        total_installments INTEGER NOT NULL,
        paid_installments INTEGER NOT NULL DEFAULT 0,
        start_date      TEXT NOT NULL,
        status          TEXT NOT NULL DEFAULT 'active',
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (employee_id) REFERENCES employees(id)
    );
    CREATE INDEX IF NOT EXISTS idx_loans_employee ON loans(employee_id);
    CREATE INDEX IF NOT EXISTS idx_loans_status ON loans(status);

    -- ============================================================
    -- LOAN PAYMENTS
    -- ============================================================
    CREATE TABLE IF NOT EXISTS loan_payments (
        id              INTEGER PRIMARY KEY,
        loan_id          INTEGER NOT NULL,
        amount          REAL NOT NULL,
        payment_date     TEXT NOT NULL,
        installment_number INTEGER NOT NULL,
        payroll_record_id INTEGER,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (loan_id) REFERENCES loans(id),
        FOREIGN KEY (payroll_record_id) REFERENCES payroll_records(id)
    );
    CREATE INDEX IF NOT EXISTS idx_loan_payments_loan ON loan_payments(loan_id);

    -- ============================================================
    -- LEAVE TYPES
    -- ============================================================
    CREATE TABLE IF NOT EXISTS leave_types (
        id              INTEGER PRIMARY KEY,
        name            TEXT NOT NULL,
        code            TEXT NOT NULL UNIQUE,
        default_days    REAL NOT NULL DEFAULT 0,
        is_paid         INTEGER NOT NULL DEFAULT 1,
        carry_forward   INTEGER NOT NULL DEFAULT 0,
        is_active       INTEGER NOT NULL DEFAULT 1,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- ============================================================
    -- LEAVE RECORDS
    -- status: 'pending' | 'approved' | 'rejected' | 'cancelled'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS leave_records (
        id              INTEGER PRIMARY KEY,
        employee_id     INTEGER NOT NULL,
        leave_type_id   INTEGER NOT NULL,
        start_date      TEXT NOT NULL,
        end_date        TEXT NOT NULL,
        days            REAL NOT NULL,
        reason          TEXT,
        status          TEXT NOT NULL DEFAULT 'pending',
        approved_by     INTEGER,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (employee_id) REFERENCES employees(id),
        FOREIGN KEY (leave_type_id) REFERENCES leave_types(id)
    );
    CREATE INDEX IF NOT EXISTS idx_leave_records_employee ON leave_records(employee_id);
    CREATE INDEX IF NOT EXISTS idx_leave_records_status ON leave_records(status);

    -- ============================================================
    -- LEAVE BALANCES (per employee per leave type per year)
    -- ============================================================
    CREATE TABLE IF NOT EXISTS leave_balances (
        id              INTEGER PRIMARY KEY,
        employee_id     INTEGER NOT NULL,
        leave_type_id   INTEGER NOT NULL,
        year            INTEGER NOT NULL,
        allocated_days  REAL NOT NULL DEFAULT 0,
        used_days       REAL NOT NULL DEFAULT 0,
        carried_days    REAL NOT NULL DEFAULT 0,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (employee_id) REFERENCES employees(id),
        FOREIGN KEY (leave_type_id) REFERENCES leave_types(id),
        UNIQUE (employee_id, leave_type_id, year)
    );
    CREATE INDEX IF NOT EXISTS idx_leave_balances_employee ON leave_balances(employee_id);

    -- ============================================================
    -- PAYSLIPS
    -- ============================================================
    CREATE TABLE IF NOT EXISTS payslips (
        id                  INTEGER PRIMARY KEY,
        payroll_record_id   INTEGER NOT NULL,
        employee_id         INTEGER NOT NULL,
        period_id           INTEGER NOT NULL,
        payslip_number      TEXT NOT NULL UNIQUE,
        net_pay             REAL NOT NULL,
        generated_at        TEXT NOT NULL DEFAULT (datetime('now')),
        pdf_path            TEXT,
        FOREIGN KEY (payroll_record_id) REFERENCES payroll_records(id),
        FOREIGN KEY (employee_id) REFERENCES employees(id),
        FOREIGN KEY (period_id) REFERENCES payroll_periods(id)
    );
    CREATE INDEX IF NOT EXISTS idx_payslips_employee ON payslips(employee_id);
    CREATE INDEX IF NOT EXISTS idx_payslips_period ON payslips(period_id);

    -- ============================================================
    -- DEVICES (registered for LAN transfer)
    -- status: 'paired' | 'revoked' | 'pending'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS devices (
        id              INTEGER PRIMARY KEY,
        device_name     TEXT NOT NULL,
        device_id       TEXT NOT NULL UNIQUE,
        ip_address      TEXT,
        pairing_code    TEXT,
        status          TEXT NOT NULL DEFAULT 'pending',
        paired_at      TEXT,
        last_seen      TEXT,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);

    -- ============================================================
    -- AUDIT LOGS
    -- ============================================================
    CREATE TABLE IF NOT EXISTS audit_logs (
        id              INTEGER PRIMARY KEY,
        action          TEXT NOT NULL,
        entity_type     TEXT,
        entity_id       INTEGER,
        details         TEXT,
        created_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_audit_logs_entity ON audit_logs(entity_type, entity_id);
    CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);

    -- ============================================================
    -- BACKUPS
    -- status: 'completed' | 'failed' | 'restoring'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS backups (
        id              INTEGER PRIMARY KEY,
        file_path       TEXT NOT NULL,
        file_size       INTEGER,
        backup_type     TEXT NOT NULL DEFAULT 'manual',
        status          TEXT NOT NULL DEFAULT 'completed',
        created_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_backups_type ON backups(backup_type);

    -- ============================================================
    -- TRANSFER HISTORY (LAN)
    -- direction: 'sent' | 'received'
    -- status: 'success' | 'failed' | 'cancelled'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS transfer_history (
        id              INTEGER PRIMARY KEY,
        device_id       INTEGER,
        direction       TEXT NOT NULL,
        file_name       TEXT NOT NULL,
        file_size       INTEGER,
        status          TEXT NOT NULL DEFAULT 'success',
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (device_id) REFERENCES devices(id)
    );
    CREATE INDEX IF NOT EXISTS idx_transfer_history_status ON transfer_history(status);

    -- ============================================================
    -- UPDATE HISTORY
    -- status: 'installed' | 'failed' | 'rolled_back'
    -- ============================================================
    CREATE TABLE IF NOT EXISTS update_history (
        id              INTEGER PRIMARY KEY,
        version_from    TEXT,
        version_to      TEXT NOT NULL,
        status          TEXT NOT NULL DEFAULT 'installed',
        created_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- ============================================================
    -- REMINDERS
    -- ============================================================
    CREATE TABLE IF NOT EXISTS reminders (
        id              INTEGER PRIMARY KEY,
        title           TEXT NOT NULL,
        message         TEXT,
        reminder_type   TEXT NOT NULL,
        due_date        TEXT NOT NULL,
        is_completed    INTEGER NOT NULL DEFAULT 0,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_reminders_due ON reminders(due_date, is_completed);
    "#,

    // Migration 003: Auth security — sessions table + security settings
    r#"
    CREATE TABLE IF NOT EXISTS admin_sessions (
        id              INTEGER PRIMARY KEY,
        admin_id        INTEGER NOT NULL,
        session_token   TEXT NOT NULL UNIQUE,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        expires_at      TEXT NOT NULL,
        last_activity   TEXT NOT NULL DEFAULT (datetime('now')),
        is_locked       INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY (admin_id) REFERENCES admin_users(id)
    );
    CREATE INDEX IF NOT EXISTS idx_admin_sessions_token ON admin_sessions(session_token);
    CREATE INDEX IF NOT EXISTS idx_admin_sessions_admin ON admin_sessions(admin_id);

    INSERT OR IGNORE INTO app_settings (key, value) VALUES
        ('auto_lock_minutes', '15'),
        ('session_timeout_minutes', '480');
    "#,

    // Migration 004: Enhanced payroll configuration — versioned rules, formula deps
    r#"
    -- Payroll rules (versioned, ordered, configurable)
    -- calc_type: 'fixed' | 'percentage' | 'formula'
    -- component_type: 'earning' | 'deduction' | 'tax' | 'contribution'
    -- side: 'employee' | 'employer'
    -- base_reference: code of component this rule is based on (for percentage), or 'gross' or 'taxable'
    CREATE TABLE IF NOT EXISTS payroll_rules (
        id                  INTEGER PRIMARY KEY,
        name                TEXT NOT NULL,
        code                TEXT NOT NULL UNIQUE,
        component_type      TEXT NOT NULL,
        calc_type           TEXT NOT NULL,
        side                TEXT NOT NULL DEFAULT 'employee',
        rate                REAL,
        formula_expression  TEXT,
        base_reference      TEXT,
        is_taxable          INTEGER NOT NULL DEFAULT 0,
        is_pensionable      INTEGER NOT NULL DEFAULT 0,
        sort_order          INTEGER NOT NULL DEFAULT 0,
        effective_date      TEXT NOT NULL DEFAULT (datetime('now')),
        is_active           INTEGER NOT NULL DEFAULT 1,
        version             INTEGER NOT NULL DEFAULT 1,
        parent_rule_id      INTEGER,
        created_at           TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at           TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (parent_rule_id) REFERENCES payroll_rules(id)
    );
    CREATE INDEX IF NOT EXISTS idx_payroll_rules_code ON payroll_rules(code);
    CREATE INDEX IF NOT EXISTS idx_payroll_rules_active ON payroll_rules(is_active);
    CREATE INDEX IF NOT EXISTS idx_payroll_rules_order ON payroll_rules(sort_order);
    CREATE INDEX IF NOT EXISTS idx_payroll_rules_version ON payroll_rules(version);

    -- Rule version history (immutable record of every rule version)
    CREATE TABLE IF NOT EXISTS payroll_rule_versions (
        id                  INTEGER PRIMARY KEY,
        rule_id             INTEGER NOT NULL,
        version             INTEGER NOT NULL,
        name                TEXT NOT NULL,
        code                TEXT NOT NULL,
        component_type      TEXT NOT NULL,
        calc_type           TEXT NOT NULL,
        side                TEXT NOT NULL,
        rate                REAL,
        formula_expression  TEXT,
        base_reference      TEXT,
        is_taxable          INTEGER NOT NULL DEFAULT 0,
        is_pensionable      INTEGER NOT NULL DEFAULT 0,
        sort_order          INTEGER NOT NULL DEFAULT 0,
        effective_date      TEXT NOT NULL,
        created_at           TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (rule_id) REFERENCES payroll_rules(id),
        UNIQUE (rule_id, version)
    );
    CREATE INDEX IF NOT EXISTS idx_rule_versions_rule ON payroll_rule_versions(rule_id);

    -- Formula dependencies (explicit ordering + circular prevention)
    CREATE TABLE IF NOT EXISTS payroll_rule_dependencies (
        id              INTEGER PRIMARY KEY,
        rule_id         INTEGER NOT NULL,
        depends_on_code TEXT NOT NULL,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (rule_id) REFERENCES payroll_rules(id)
    );
    CREATE INDEX IF NOT EXISTS idx_rule_deps_rule ON payroll_rule_dependencies(rule_id);

    -- Seed reference components as editable configuration (NOT hard-coded rules)
    INSERT OR IGNORE INTO payroll_rules (name, code, component_type, calc_type, side, rate, base_reference, sort_order, is_active, version) VALUES
        ('Basic Salary',         'BASIC',         'earning',     'fixed',      'employee', NULL, NULL,           1,  1, 1),
        ('Transport',            'TRANSPORT',     'earning',     'percentage', 'employee', 8,   'BASIC',        2,  1, 1),
        ('Accommodation',        'ACCOMMODATION', 'earning',     'percentage', 'employee', 12,  'BASIC',        3,  1, 1),
        ('Taxable Base',         'TAXABLE_BASE',  'earning',     'formula',    'employee', NULL, NULL,           4,  1, 1),
        ('PAYE',                 'PAYE',          'tax',         'formula',    'employee', NULL, NULL,           5,  1, 1),
        ('Employee Pension',     'PENSION_EMP',   'contribution','percentage', 'employee', 6,   'BASIC',        6,  1, 1),
        ('Employer Pension',     'PENSION_ER',    'contribution','percentage', 'employer', 6,   'BASIC',        7,  1, 1),
        ('Pension 2%',           'PENSION_2',     'contribution','percentage', 'employee', 2,   'BASIC',        8,  1, 1),
        ('Maternity Employee',   'MATERNITY_EMP', 'contribution','percentage', 'employee', 0.3, 'BASIC',        9,  1, 1),
        ('Maternity Employer',   'MATERNITY_ER',  'contribution','percentage', 'employer', 0.3, 'BASIC',        10, 1, 1),
        ('CHBI',                 'CHBI',          'contribution','percentage', 'employee', 0.5, 'BASIC',        11, 1, 1),
        ('Other Deductions',     'OTHER_DED',     'deduction',   'fixed',      'employee', 0,   NULL,           12, 1, 1),
        ('Total Deductions',     'TOTAL_DED',     'deduction',   'formula',    'employee', NULL, NULL,           13, 1, 1),
        ('Net Salary',           'NET_SALARY',    'earning',     'formula',    'employee', NULL, NULL,           14, 1, 1);

    -- Seed formula expressions for formula-type rules
    UPDATE OR IGNORE payroll_rules SET formula_expression = 'BASIC + TRANSPORT + ACCOMMODATION' WHERE code = 'TAXABLE_BASE' AND formula_expression IS NULL;
    UPDATE OR IGNORE payroll_rules SET formula_expression = 'TAXABLE_BASE * 0.1' WHERE code = 'PAYE' AND formula_expression IS NULL;
    UPDATE OR IGNORE payroll_rules SET formula_expression = 'PAYE + PENSION_EMP + PENSION_2 + MATERNITY_EMP + CHBI + OTHER_DED' WHERE code = 'TOTAL_DED' AND formula_expression IS NULL;
    UPDATE OR IGNORE payroll_rules SET formula_expression = 'TAXABLE_BASE - TOTAL_DED' WHERE code = 'NET_SALARY' AND formula_expression IS NULL;

    -- Seed formula dependencies
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'BASIC' FROM payroll_rules WHERE code = 'TRANSPORT';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'BASIC' FROM payroll_rules WHERE code = 'ACCOMMODATION';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'BASIC' FROM payroll_rules WHERE code = 'TRANSPORT';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'BASIC' FROM payroll_rules WHERE code = 'ACCOMMODATION';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'BASIC' FROM payroll_rules WHERE code = 'TAXABLE_BASE';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'TRANSPORT' FROM payroll_rules WHERE code = 'TAXABLE_BASE';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'ACCOMMODATION' FROM payroll_rules WHERE code = 'TAXABLE_BASE';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'TAXABLE_BASE' FROM payroll_rules WHERE code = 'PAYE';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'PAYE' FROM payroll_rules WHERE code = 'TOTAL_DED';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'PENSION_EMP' FROM payroll_rules WHERE code = 'TOTAL_DED';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'PENSION_2' FROM payroll_rules WHERE code = 'TOTAL_DED';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'MATERNITY_EMP' FROM payroll_rules WHERE code = 'TOTAL_DED';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'CHBI' FROM payroll_rules WHERE code = 'TOTAL_DED';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'OTHER_DED' FROM payroll_rules WHERE code = 'TOTAL_DED';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'TAXABLE_BASE' FROM payroll_rules WHERE code = 'NET_SALARY';
    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'TOTAL_DED' FROM payroll_rules WHERE code = 'NET_SALARY';

    -- Seed version history for initial rules
    INSERT OR IGNORE INTO payroll_rule_versions (rule_id, version, name, code, component_type, calc_type, side, rate, formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date)
        SELECT id, version, name, code, component_type, calc_type, side, rate, formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date FROM payroll_rules;
    "#,

    // Migration 005: Staff module — add employee fields
    r#"
    ALTER TABLE employees ADD COLUMN grade TEXT;
    ALTER TABLE employees ADD COLUMN dependants INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE employees ADD COLUMN rssb_number TEXT;
    "#,

    // Migration 002: Seed minimum system data
    r#"
    INSERT OR IGNORE INTO app_settings (key, value) VALUES
        ('app_name', 'Payroll System'),
        ('app_version', '1.0.0'),
        ('currency', 'USD'),
        ('language', 'en'),
        ('theme', 'dark');

    INSERT OR IGNORE INTO salary_components (name, code, component_type, calc_type, default_value, applies_to) VALUES
        ('Basic Salary', 'BASIC', 'earning', 'fixed', 0, 'all'),
        ('Transport Allowance', 'TRANSPORT', 'earning', 'percentage', 8, 'all'),
        ('Housing Allowance', 'HOUSING', 'earning', 'percentage', 12, 'all'),
        ('Pension Contribution', 'PENSION', 'contribution', 'percentage', 6, 'all'),
        ('Tax', 'TAX', 'tax', 'formula', NULL, 'all');

    INSERT OR IGNORE INTO leave_types (name, code, default_days, is_paid, carry_forward) VALUES
        ('Annual Leave', 'ANNUAL', 21, 1, 1),
        ('Sick Leave', 'SICK', 10, 1, 0),
        ('Unpaid Leave', 'UNPAID', 0, 0, 0);
    "#,
    // Migration 006: Offline Admin recovery code
    r#"
    ALTER TABLE admin_users ADD COLUMN recovery_hash TEXT;
    "#,

    // Migration 007: Extended company profile fields
    r#"
    ALTER TABLE companies ADD COLUMN website TEXT;
    ALTER TABLE companies ADD COLUMN tin_number TEXT;
    ALTER TABLE companies ADD COLUMN rssb_number TEXT;
    "#,

    // Migration 008: Staff contract types, employee photo, and contract reference
    r#"
    CREATE TABLE IF NOT EXISTS contract_types (
        id              INTEGER PRIMARY KEY,
        name            TEXT NOT NULL UNIQUE,
        code            TEXT UNIQUE,
        description     TEXT,
        is_active       INTEGER NOT NULL DEFAULT 1,
        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
    );

    ALTER TABLE employees ADD COLUMN contract_type_id INTEGER;
    ALTER TABLE employees ADD COLUMN photo_path TEXT;

    CREATE INDEX IF NOT EXISTS idx_employees_contract_type ON employees(contract_type_id);
    CREATE INDEX IF NOT EXISTS idx_contract_types_active ON contract_types(is_active);
    "#,

    // Migration 009: Payroll reference configuration aligned to the supplied
    // Mountain Gorilla Veterinary payroll report. Existing customized rules
    // (version > 1) are preserved.
    r#"
    UPDATE payroll_rules
       SET rate = 10, base_reference = 'BASIC', version = version + 1, updated_at = datetime('now')
     WHERE code = 'TRANSPORT' AND version = 1;

    UPDATE payroll_rules
       SET rate = 15, base_reference = 'BASIC', version = version + 1, updated_at = datetime('now')
     WHERE code = 'ACCOMMODATION' AND version = 1;

    UPDATE payroll_rules
       SET formula_expression = 'max(0, min((TAXABLE_BASE - 80000) * 0.2, 24000) + max(0, TAXABLE_BASE - 200000) * 0.3)',
           version = version + 1, updated_at = datetime('now')
     WHERE code = 'PAYE' AND version = 1;

    UPDATE payroll_rules
       SET base_reference = 'TAXABLE_BASE', version = version + 1, updated_at = datetime('now')
     WHERE code IN ('PENSION_EMP','PENSION_ER','PENSION_2','MATERNITY_EMP','MATERNITY_ER') AND version = 1;

    UPDATE payroll_rules
       SET side = 'employer', base_reference = 'TAXABLE_BASE', updated_at = datetime('now')
     WHERE code = 'PENSION_2' AND version IN (1,2);

    UPDATE payroll_rules
       SET side = 'employer', base_reference = 'NET_SALARY', version = version + 1, updated_at = datetime('now')
     WHERE code = 'CHBI' AND version = 1;

    UPDATE payroll_rules
       SET formula_expression = 'PAYE + PENSION_EMP + MATERNITY_EMP + OTHER_DED + LOAN_DED',
           version = version + 1, updated_at = datetime('now')
     WHERE code = 'TOTAL_DED' AND version = 1;

    UPDATE payroll_rules
       SET formula_expression = 'TAXABLE_BASE - TOTAL_DED', version = version + 1, updated_at = datetime('now')
     WHERE code = 'NET_SALARY' AND version = 1;

    INSERT OR IGNORE INTO payroll_rules
        (name, code, component_type, calc_type, side, rate, formula_expression, base_reference, sort_order, is_active, version)
    VALUES
        ('Loan / Advance Deduction', 'LOAN_DED', 'deduction', 'fixed', 'employee', NULL, NULL, NULL, 12, 1, 1);

    UPDATE payroll_rules SET sort_order = 13 WHERE code = 'OTHER_DED' AND sort_order = 12;
    UPDATE payroll_rules SET sort_order = 14 WHERE code = 'TOTAL_DED' AND sort_order = 13;
    UPDATE payroll_rules SET sort_order = 15 WHERE code = 'NET_SALARY' AND sort_order = 14;

    INSERT OR IGNORE INTO payroll_rule_dependencies (rule_id, depends_on_code)
        SELECT id, 'LOAN_DED' FROM payroll_rules WHERE code = 'TOTAL_DED';

    INSERT OR IGNORE INTO payroll_rule_versions
        (rule_id, version, name, code, component_type, calc_type, side, rate, formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date)
        SELECT id, version, name, code, component_type, calc_type, side, rate, formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date
        FROM payroll_rules WHERE code IN ('TRANSPORT','ACCOMMODATION','PAYE','PENSION_EMP','PENSION_ER','PENSION_2','MATERNITY_EMP','MATERNITY_ER','CHBI','TOTAL_DED','NET_SALARY','LOAN_DED');
    "#,

    // Migration 010: Preserve the supplied Mountain Gorilla report as the initial
    // active payroll configuration without overwriting later Admin customizations.
    r#"
    UPDATE salary_components SET default_value = 10 WHERE code = 'TRANSPORT';
    UPDATE salary_components SET default_value = 15 WHERE code = 'HOUSING';

    UPDATE payroll_rules
       SET rate = 10, base_reference = 'BASIC', version = version + 1, updated_at = datetime('now')
     WHERE code = 'TRANSPORT' AND version <= 2;

    UPDATE payroll_rules
       SET rate = 15, base_reference = 'BASIC', version = version + 1, updated_at = datetime('now')
     WHERE code = 'ACCOMMODATION' AND version <= 2;

    UPDATE payroll_rules
       SET formula_expression = 'BASIC + TRANSPORT + ACCOMMODATION', version = version + 1, updated_at = datetime('now')
     WHERE code = 'TAXABLE_BASE' AND version <= 3;

    UPDATE payroll_rules
       SET formula_expression = 'max(0, min((TAXABLE_BASE - 80000) * 0.2, 24000) + max(0, TAXABLE_BASE - 200000) * 0.3)', version = version + 1, updated_at = datetime('now')
     WHERE code = 'PAYE' AND version <= 3;

    UPDATE payroll_rules
       SET base_reference = 'TAXABLE_BASE', version = version + 1, updated_at = datetime('now')
     WHERE code IN ('PENSION_EMP','PENSION_ER','PENSION_2','MATERNITY_EMP','MATERNITY_ER') AND version <= 3;

    UPDATE payroll_rules
       SET side = 'employer', base_reference = 'TAXABLE_BASE', version = version + 1, updated_at = datetime('now')
     WHERE code = 'PENSION_2' AND version <= 4;

    UPDATE payroll_rules
       SET formula_expression = 'PAYE + PENSION_EMP + MATERNITY_EMP + OTHER_DED + LOAN_DED', version = version + 1, updated_at = datetime('now')
     WHERE code = 'TOTAL_DED' AND version <= 3;

    UPDATE payroll_rules
       SET formula_expression = 'TAXABLE_BASE - TOTAL_DED', version = version + 1, updated_at = datetime('now')
     WHERE code = 'NET_SALARY' AND version <= 3;

    UPDATE payroll_rules
       SET side = 'employer', base_reference = 'NET_SALARY', rate = 0.5, version = version + 1, updated_at = datetime('now')
     WHERE code = 'CHBI' AND version <= 3;

    INSERT OR IGNORE INTO payroll_rule_versions
        (rule_id, version, name, code, component_type, calc_type, side, rate, formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date)
        SELECT id, version, name, code, component_type, calc_type, side, rate, formula_expression, base_reference, is_taxable, is_pensionable, sort_order, effective_date
        FROM payroll_rules
        WHERE code IN ('BASIC','TRANSPORT','ACCOMMODATION','TAXABLE_BASE','PAYE','PENSION_EMP','PENSION_ER','PENSION_2','MATERNITY_EMP','MATERNITY_ER','CHBI','OTHER_DED','TOTAL_DED','NET_SALARY','LOAN_DED');
    "#,
    // Migration 011: Per-employee payroll component overrides
    r#"
    CREATE TABLE IF NOT EXISTS employee_payroll_overrides (
        id               INTEGER PRIMARY KEY,
        employee_id      INTEGER NOT NULL,
        rule_id          INTEGER NOT NULL,
        override_type    TEXT NOT NULL CHECK (override_type IN ('fixed','percentage','formula')),
        value            REAL,
        formula_expression TEXT,
        base_reference   TEXT,
        effective_date   TEXT NOT NULL DEFAULT (date('now')),
        is_active        INTEGER NOT NULL DEFAULT 1,
        created_at       TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at       TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE(employee_id, rule_id, effective_date),
        FOREIGN KEY (employee_id) REFERENCES employees(id),
        FOREIGN KEY (rule_id) REFERENCES payroll_rules(id)
    );
    CREATE INDEX IF NOT EXISTS idx_employee_payroll_overrides_employee ON employee_payroll_overrides(employee_id);
    CREATE INDEX IF NOT EXISTS idx_employee_payroll_overrides_rule ON employee_payroll_overrides(rule_id);
    CREATE INDEX IF NOT EXISTS idx_employee_payroll_overrides_active ON employee_payroll_overrides(is_active);
    "#,

    // Migration 012: Payroll idempotency and duplicate protection
    r#"
    -- Keep the newest payroll record for each employee/period and remove older duplicates.
    DELETE FROM payroll_items
     WHERE payroll_record_id IN (
       SELECT r.id
       FROM payroll_records r
       WHERE EXISTS (
         SELECT 1 FROM payroll_records newer
          WHERE newer.period_id = r.period_id
            AND newer.employee_id = r.employee_id
            AND newer.id > r.id
       )
     );

    DELETE FROM payslips
     WHERE payroll_record_id IN (
       SELECT r.id
       FROM payroll_records r
       WHERE EXISTS (
         SELECT 1 FROM payroll_records newer
          WHERE newer.period_id = r.period_id
            AND newer.employee_id = r.employee_id
            AND newer.id > r.id
       )
     );

    DELETE FROM payroll_records
     WHERE EXISTS (
       SELECT 1 FROM payroll_records newer
        WHERE newer.period_id = payroll_records.period_id
          AND newer.employee_id = payroll_records.employee_id
          AND newer.id > payroll_records.id
     );

    -- Keep the newest payslip for each employee/period.
    DELETE FROM payslips
     WHERE EXISTS (
       SELECT 1 FROM payslips newer
        WHERE newer.period_id = payslips.period_id
          AND newer.employee_id = payslips.employee_id
          AND newer.id > payslips.id
     );

    CREATE UNIQUE INDEX IF NOT EXISTS uq_payroll_records_period_employee
      ON payroll_records(period_id, employee_id);
    CREATE UNIQUE INDEX IF NOT EXISTS uq_payslips_period_employee
      ON payslips(period_id, employee_id);
    CREATE UNIQUE INDEX IF NOT EXISTS uq_payslips_payroll_record
      ON payslips(payroll_record_id);
    "#,

    // Migration 013: Backup configuration and metadata
    r#"
    ALTER TABLE backups ADD COLUMN checksum TEXT;
    ALTER TABLE backups ADD COLUMN encrypted INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE backups ADD COLUMN database_version TEXT;
    ALTER TABLE backups ADD COLUMN app_version TEXT;

    INSERT OR IGNORE INTO app_settings (key, value) VALUES
        ('backup_enabled', '1'),
        ('backup_frequency', 'daily'),
        ('backup_time', '02:00'),
        ('backup_retention', '7'),
        ('backup_location', '');
    "#,

    // Migration 014: Reminder scheduling and lifecycle fields
    r#"
    ALTER TABLE reminders ADD COLUMN recurrence TEXT NOT NULL DEFAULT 'none';
    ALTER TABLE reminders ADD COLUMN snoozed_until TEXT;
    ALTER TABLE reminders ADD COLUMN completed_at TEXT;
    CREATE INDEX IF NOT EXISTS idx_reminders_active_due ON reminders(is_completed, due_date, snoozed_until);
    "#,

    // Migration 015: Reminder notification read state
    r#"
    ALTER TABLE reminders ADD COLUMN read_at TEXT;
    CREATE INDEX IF NOT EXISTS idx_reminders_notification ON reminders(is_completed, due_date, snoozed_until, read_at);
    "#,
    // Migration 016: Settings page — general + update preferences
    r#"
    INSERT OR IGNORE INTO app_settings (key, value) VALUES
        ('date_format', 'DD/MM/YYYY'),
        ('number_format', '1,234.56'),
        ('payroll_period_behavior', 'monthly'),
        ('update_auto_check', '1'),
        ('update_channel', 'stable');
    "#,

    // Migration 017: Extend the existing update history with auditable lifecycle events.
    r#"
    ALTER TABLE update_history ADD COLUMN event_type TEXT;
    ALTER TABLE update_history ADD COLUMN details TEXT;
    ALTER TABLE update_history ADD COLUMN size_bytes INTEGER;
    CREATE INDEX IF NOT EXISTS idx_update_history_event ON update_history(event_type, created_at);
    INSERT OR IGNORE INTO app_settings (key, value) VALUES ('update_last_check_epoch', '0');
    "#,

    // Migration 018: Notification center read state for system events.
    r#"
    CREATE TABLE IF NOT EXISTS notification_reads (
        audit_id INTEGER PRIMARY KEY,
        read_at TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (audit_id) REFERENCES audit_logs(id) ON DELETE CASCADE
    );
    CREATE INDEX IF NOT EXISTS idx_notification_reads_at ON notification_reads(read_at);
    "#,

    // Migration 019: Reminder and notification preferences.
    r#"
    INSERT OR IGNORE INTO app_settings (key, value) VALUES
        ('reminder_notifications_enabled', '1'),
        ('reminder_polling_interval_seconds', '15'),
        ('reminder_default_snooze_minutes', '60'),
        ('reminder_default_category', 'general'),
        ('reminder_show_completed', '1'),
        ('reminder_auto_advance_recurring', '1');
    "#,

];

pub fn run(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let latest: Option<i64> = conn
        .query_row(
            "SELECT MAX(version) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .ok();

    let start = latest.unwrap_or(0) as usize;

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version <= start as i64 {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [version],
        )?;
    }

    Ok(())
}
