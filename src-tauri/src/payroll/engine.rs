use std::collections::{HashMap, HashSet};

use evalexpr::*;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::models::*;

pub fn detect_circular(rules: &[PayrollRule]) -> Result<(), String> {
    let code_to_rules: HashMap<&str, &PayrollRule> = rules.iter().map(|r| (r.code.as_str(), r)).collect();

    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    for rule in rules {
        let deps = extract_deps(rule);
        for dep in deps {
            if code_to_rules.contains_key(dep.as_str()) {
                graph.entry(rule.code.as_str()).or_default().push(dep.leak());
            }
        }
    }

    let mut visited = HashSet::new();
    let mut stack = HashSet::new();

    for rule in rules {
        if visit(rule.code.as_str(), &graph, &mut visited, &mut stack)? {
            return Err(format!("Circular dependency detected involving '{}'", rule.code));
        }
    }

    Ok(())
}

fn visit<'a>(
    node: &'a str,
    graph: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    stack: &mut HashSet<&'a str>,
) -> Result<bool, String> {
    if stack.contains(node) {
        return Ok(true);
    }
    if visited.contains(node) {
        return Ok(false);
    }

    visited.insert(node);
    stack.insert(node);

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if visit(dep, graph, visited, stack)? {
                return Ok(true);
            }
        }
    }

    stack.remove(node);
    Ok(false)
}

fn extract_deps(rule: &PayrollRule) -> Vec<String> {
    if rule.calc_type != "formula" {
        if let Some(base) = &rule.base_reference {
            return vec![base.clone()];
        }
        return vec![];
    }

    let mut deps = Vec::new();
    if let Some(expr) = &rule.formula_expression {
        let mut current = String::new();
        for ch in expr.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                current.push(ch);
            } else if !current.is_empty() {
                if current.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && !is_builtin(&current)
                    && !deps.contains(&current)
                {
                    deps.push(current.clone());
                }
                current.clear();
            }
        }
        if !current.is_empty()
            && current.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && !is_builtin(&current)
            && !deps.contains(&current)
        {
            deps.push(current);
        }
    }

    if let Some(base) = &rule.base_reference {
        if !deps.contains(base) {
            deps.push(base.clone());
        }
    }

    deps
}

fn is_builtin(id: &str) -> bool {
    matches!(id, "true" | "false" | "min" | "max" | "round" | "floor" | "ceil" | "abs")
}

fn topological_sort(rules: &[PayrollRule]) -> Result<Vec<&PayrollRule>, String> {
    let code_to_rule: HashMap<String, &PayrollRule> =
        rules.iter().map(|r| (r.code.clone(), r)).collect();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    for rule in rules {
        in_degree.entry(rule.code.clone()).or_insert(0);
        graph.entry(rule.code.clone()).or_default();
    }

    for rule in rules {
        for dep in extract_deps(rule) {
            if code_to_rule.contains_key(&dep) {
                graph.entry(dep).or_default().push(rule.code.clone());
                *in_degree.entry(rule.code.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| k.clone())
        .collect();
    queue.sort();

    let mut result = Vec::with_capacity(rules.len());
    while let Some(node) = queue.pop() {
        if let Some(rule) = code_to_rule.get(&node) {
            result.push(*rule);
        }
        if let Some(neighbors) = graph.get(&node) {
            for neighbor in neighbors {
                if let Some(d) = in_degree.get_mut(neighbor) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push(neighbor.clone());
                        queue.sort();
                    }
                }
            }
        }
    }

    if result.len() != rules.len() {
        return Err("Circular dependency detected during sorting".to_string());
    }

    Ok(result)
}

pub fn calculate(rules: &[PayrollRule], inputs: &HashMap<String, Decimal>) -> CalcResult {
    let mut values: HashMap<String, Decimal> = inputs.clone();
    let mut items: Vec<CalcItem> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let sorted = match topological_sort(rules) {
        Ok(s) => s,
        Err(e) => {
            return CalcResult {
                items: vec![],
                gross_earnings: "0".to_string(),
                total_deductions: "0".to_string(),
                total_tax: "0".to_string(),
                net_pay: "0".to_string(),
                employer_contributions: "0".to_string(),
                errors: vec![e],
            };
        }
    };

    for rule in sorted {
        let amount = match compute_rule(rule, &values) {
            Ok(a) => a,
            Err(e) => {
                errors.push(format!("{}: {}", rule.code, e));
                rust_decimal_macros::dec!(0)
            }
        };

        values.insert(rule.code.clone(), amount);

        items.push(CalcItem {
            code: rule.code.clone(),
            name: rule.name.clone(),
            component_type: rule.component_type.clone(),
            calc_type: rule.calc_type.clone(),
            side: rule.side.clone(),
            rate: rule.rate,
            formula: rule.formula_expression.clone(),
            base_reference: rule.base_reference.clone(),
            amount: amount.to_string(),
        });
    }

    let gross = items
        .iter()
        .find(|i| i.code == "TAXABLE_BASE")
        .and_then(|i| Decimal::from_str(&i.amount).ok())
        .unwrap_or_else(|| {
            items
                .iter()
                .filter(|i| i.component_type == "earning" && i.side == "employee" && i.calc_type != "formula")
                .filter_map(|i| Decimal::from_str(&i.amount).ok())
                .fold(dec!(0), |acc, value| acc + value)
        });
    let total_ded = sum_by_type(&items, "deduction", "employee")
        + sum_by_type(&items, "contribution", "employee")
        + sum_by_type(&items, "tax", "employee");
    let total_tax = sum_by_type(&items, "tax", "employee");
    let employer = sum_by_type(&items, "contribution", "employer");

    let net = items
        .iter()
        .find(|i| i.code == "NET_SALARY")
        .and_then(|i| Decimal::from_str(&i.amount).ok())
        .unwrap_or_else(|| gross - total_ded);

    CalcResult {
        items,
        gross_earnings: gross.to_string(),
        total_deductions: total_ded.to_string(),
        total_tax: total_tax.to_string(),
        net_pay: net.to_string(),
        employer_contributions: employer.to_string(),
        errors,
    }
}

fn compute_rule(rule: &PayrollRule, values: &HashMap<String, Decimal>) -> Result<Decimal, String> {
    match rule.calc_type.as_str() {
        "fixed" => {
            if let Some(rate) = rule.rate {
                Ok(Decimal::from_f64(rate).unwrap_or(dec!(0)))
            } else {
                Ok(values.get(&rule.code).cloned().unwrap_or(dec!(0)))
            }
        }
        "percentage" => {
            let base_code = rule.base_reference.as_deref().unwrap_or("BASIC");
            let base = values.get(base_code).cloned().unwrap_or(dec!(0));
            let rate = Decimal::from_str_exact(&rule.rate.unwrap_or(0.0).to_string()).unwrap_or(dec!(0));
            let hundred = dec!(100);
            Ok(base * rate / hundred)
        }
        "formula" => {
            let expr = rule
                .formula_expression
                .as_ref()
                .ok_or_else(|| format!("No formula for {}", rule.code))?;

            let context = build_context(values);

            match eval_expression(expr, &context) {
                Ok(Value::Float(f)) => Ok(Decimal::from_f64(f).unwrap_or(dec!(0))),
                Ok(Value::Int(i)) => Ok(Decimal::from(i)),
                Ok(_) => Err(format!("Formula {} returned non-numeric result", rule.code)),
                Err(e) => Err(format!("Formula error in {}: {}", rule.code, e)),
            }
        }
        _ => Err(format!("Unknown calc_type: {}", rule.calc_type)),
    }
}

fn sum_by_type(items: &[CalcItem], component_type: &str, side: &str) -> Decimal {
    items
        .iter()
        .filter(|i| i.component_type == component_type && i.side == side)
        .filter_map(|i| Decimal::from_str(&i.amount).ok())
        .fold(dec!(0), |acc, v| acc + v)
}


fn build_context(values: &HashMap<String, Decimal>) -> HashMapContext {
    let mut context = HashMapContext::new();
    for (key, val) in values {
        let f = val.to_string().parse::<f64>().unwrap_or(0.0);
        let _ = context.set_value(key.clone(), Value::Float(f));
    }
    context
}

fn eval_expression(expr: &str, context: &HashMapContext) -> Result<Value, String> {
    let normalized = expr.trim();
    if let Some((start, end, name, inside)) = find_innermost_function(normalized) {
        let parts = split_top_level_args(inside);
        if parts.len() != 2 {
            return Err(format!("{}() expects two numeric arguments", name));
        }
        let left = eval_expression(parts[0], context)?;
        let right = eval_expression(parts[1], context)?;
        let left_num = left.as_number().map_err(|e| e.to_string())?;
        let right_num = right.as_number().map_err(|e| e.to_string())?;
        let value = match name {
            "max" => left_num.max(right_num),
            "min" => left_num.min(right_num),
            _ => return Err(format!("Unsupported function '{}'", name)),
        };

        let mut reduced = String::new();
        reduced.push_str(&normalized[..start]);
        reduced.push_str(&value.to_string());
        reduced.push_str(&normalized[end..]);
        return match evalexpr::eval_with_context(&reduced, context) {
            Ok(value) => Ok(value),
            Err(e) => Err(e.to_string()),
        };
    }

    evalexpr::eval_with_context(normalized, context).map_err(|e| e.to_string())
}

fn find_innermost_function(expr: &str) -> Option<(usize, usize, &str, &str)> {
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let starts = expr[i..].starts_with("max(") || expr[i..].starts_with("min(");
        if starts {
            let name = if expr[i..].starts_with("max(") { "max" } else { "min" };
            let open = i + name.len();
            let mut depth = 0i32;
            let mut j = open;
            while j < bytes.len() {
                if bytes[j] == b'(' { depth += 1; }
                if bytes[j] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        return Some((i, j + 1, name, &expr[open + 1..j]));
                    }
                }
                j += 1;
            }
            return None;
        }
        i += 1;
    }
    None
}

fn split_top_level_args(value: &str) -> Vec<&str> {
    let bytes = value.as_bytes();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut parts = Vec::new();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(value[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(value[start..].trim());
    parts
}

pub fn test_formula(
    expr: &str,
    inputs: &[TestInput],
    rules: &[PayrollRule],
) -> FormulaTestResult {
    let mut values: HashMap<String, Decimal> = HashMap::new();
    for input in inputs {
        values.insert(input.code.clone(), Decimal::from_f64(input.value).unwrap_or(dec!(0)));
    }

    for rule in rules {
        if !values.contains_key(&rule.code) {
            values.insert(rule.code.clone(), dec!(0));
        }
    }

    let context = build_context(&values);

    match eval_expression(expr, &context) {
        Ok(Value::Float(f)) => {
            let result = Decimal::from_f64(f).unwrap_or(dec!(0)).round_dp(2);
            let mut breakdown = Vec::new();
            for input in inputs {
                breakdown.push(CalcItem {
                    code: input.code.clone(),
                    name: input.code.clone(),
                    component_type: "input".to_string(),
                    calc_type: "input".to_string(),
                    side: "employee".to_string(),
                    rate: None,
                    formula: None,
                    base_reference: None,
                    amount: Decimal::from_f64(input.value).unwrap_or(dec!(0)).to_string(),
                });
            }
            breakdown.push(CalcItem {
                code: "RESULT".to_string(),
                name: "Formula Result".to_string(),
                component_type: "result".to_string(),
                calc_type: "formula".to_string(),
                side: "employee".to_string(),
                rate: None,
                formula: Some(expr.to_string()),
                base_reference: None,
                amount: result.to_string(),
            });
            FormulaTestResult {
                success: true,
                result: Some(result.to_string()),
                error: None,
                breakdown,
            }
        }
        Ok(Value::Int(i)) => {
            let result = Decimal::from(i);
            FormulaTestResult {
                success: true,
                result: Some(result.to_string()),
                error: None,
                breakdown: vec![CalcItem {
                    code: "RESULT".to_string(),
                    name: "Formula Result".to_string(),
                    component_type: "result".to_string(),
                    calc_type: "formula".to_string(),
                    side: "employee".to_string(),
                    rate: None,
                    formula: Some(expr.to_string()),
                    base_reference: None,
                    amount: result.to_string(),
                }],
            }
        }
        Ok(_) => FormulaTestResult {
            success: false,
            result: None,
            error: Some("Formula returned a non-numeric value".to_string()),
            breakdown: vec![],
        },
        Err(e) => FormulaTestResult {
            success: false,
            result: None,
            error: Some(format!("{}", e)),
            breakdown: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(code: &str, name: &str, component_type: &str, calc_type: &str, side: &str, rate: Option<f64>, formula: Option<&str>, base: Option<&str>, sort_order: i64) -> PayrollRule {
        PayrollRule {
            id: sort_order,
            name: name.to_string(),
            code: code.to_string(),
            component_type: component_type.to_string(),
            calc_type: calc_type.to_string(),
            side: side.to_string(),
            rate,
            formula_expression: formula.map(str::to_string),
            base_reference: base.map(str::to_string),
            is_taxable: false,
            is_pensionable: false,
            sort_order,
            effective_date: "2026-01-01".to_string(),
            is_active: true,
            version: 1,
        }
    }

    #[test]
    fn mountain_gorilla_reference_row_matches_report() {
        let rules = vec![
            rule("BASIC", "Basic Salary", "earning", "fixed", "employee", None, None, None, 1),
            rule("TRANSPORT", "Transport", "earning", "percentage", "employee", Some(10.0), None, Some("BASIC"), 2),
            rule("ACCOMMODATION", "Accommodation", "earning", "percentage", "employee", Some(15.0), None, Some("BASIC"), 3),
            rule("TAXABLE_BASE", "Taxable Base", "earning", "formula", "employee", None, Some("BASIC + TRANSPORT + ACCOMMODATION"), None, 4),
            rule("PAYE", "PAYE", "tax", "formula", "employee", None, Some("max(0, min((TAXABLE_BASE - 80000) * 0.2, 24000) + max(0, TAXABLE_BASE - 200000) * 0.3)"), None, 5),
            rule("PENSION_EMP", "Employee Pension", "contribution", "percentage", "employee", Some(6.0), None, Some("TAXABLE_BASE"), 6),
            rule("PENSION_ER", "Employer Pension", "contribution", "percentage", "employer", Some(6.0), None, Some("TAXABLE_BASE"), 7),
            rule("PENSION_2", "Pension 2%", "contribution", "percentage", "employer", Some(2.0), None, Some("TAXABLE_BASE"), 8),
            rule("MATERNITY_EMP", "Maternity Employee", "contribution", "percentage", "employee", Some(0.3), None, Some("TAXABLE_BASE"), 9),
            rule("MATERNITY_ER", "Maternity Employer", "contribution", "percentage", "employer", Some(0.3), None, Some("TAXABLE_BASE"), 10),
            rule("CHBI", "CHBI", "contribution", "percentage", "employer", Some(0.5), None, Some("NET_SALARY"), 11),
            rule("OTHER_DED", "Other Deductions", "deduction", "fixed", "employee", Some(0.0), None, None, 12),
            rule("TOTAL_DED", "Total Deductions", "deduction", "formula", "employee", None, Some("PAYE + PENSION_EMP + MATERNITY_EMP + OTHER_DED + LOAN_DED"), None, 13),
            rule("NET_SALARY", "Net Salary", "earning", "formula", "employee", None, Some("TAXABLE_BASE - TOTAL_DED"), None, 14),
            rule("LOAN_DED", "Loan / Advance Deduction", "deduction", "fixed", "employee", Some(0.0), None, None, 15),
        ];
        let mut inputs = HashMap::new();
        inputs.insert("BASIC".to_string(), Decimal::from_str_exact("3851176.8").unwrap());
        inputs.insert("LOAN_DED".to_string(), dec!(0));
        let result = calculate(&rules, &inputs);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.items.iter().find(|i| i.code == "TAXABLE_BASE").unwrap().amount, "4813971");
        assert_eq!(result.items.iter().find(|i| i.code == "PAYE").unwrap().amount, "1408191.3");
        assert_eq!(result.items.iter().find(|i| i.code == "NET_SALARY").unwrap().amount, "3102499.527");
        assert_eq!(result.items.iter().find(|i| i.code == "CHBI").unwrap().amount, "15512.497635");
    }
}
