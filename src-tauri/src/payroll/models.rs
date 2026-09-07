use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PayrollRule {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub component_type: String,
    pub calc_type: String,
    pub side: String,
    pub rate: Option<f64>,
    pub formula_expression: Option<String>,
    pub base_reference: Option<String>,
    pub is_taxable: bool,
    pub is_pensionable: bool,
    pub sort_order: i64,
    pub effective_date: String,
    pub is_active: bool,
    pub version: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CalcItem {
    pub code: String,
    pub name: String,
    pub component_type: String,
    pub calc_type: String,
    pub side: String,
    pub rate: Option<f64>,
    pub formula: Option<String>,
    pub base_reference: Option<String>,
    pub amount: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct CalcResult {
    pub items: Vec<CalcItem>,
    pub gross_earnings: String,
    pub total_deductions: String,
    pub total_tax: String,
    pub net_pay: String,
    pub employer_contributions: String,
    pub errors: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct FormulaTestResult {
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
    pub breakdown: Vec<CalcItem>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TestInput {
    pub code: String,
    pub value: f64,
}
