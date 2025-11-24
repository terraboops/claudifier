//! Event matching logic.
//!
//! This module provides functionality to match events against configured rules.

use crate::config::{MatchRules, MatchType};
use crate::event::Event;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Checks if an event matches the given rules.
///
/// # Arguments
///
/// * `event` - The event to match against
/// * `rules` - The matching rules (None matches all events)
/// * `match_type` - The type of matching to use for strings (exact or regex)
///
/// # Examples
///
/// ```
/// use boopifier::matcher::matches;
/// use boopifier::event::Event;
/// use boopifier::config::{MatchRules, MatchType};
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let event = Event::from_json(r#"{"event_type": "success", "tool": "bash"}"#).unwrap();
///
/// let mut rules = HashMap::new();
/// rules.insert("event_type".to_string(), json!("success"));
/// assert!(matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Exact));
/// ```
pub fn matches(event: &Event, rules: &Option<MatchRules>, match_type: &MatchType) -> bool {
    match rules {
        None => true, // No rules means match all
        Some(MatchRules::Simple(simple_rules)) => {
            // Check if this is actually a complex rule that was mis-deserialized
            // (happens because untagged enums try Simple first)
            if simple_rules.contains_key("any") || simple_rules.contains_key("all") || simple_rules.contains_key("not") {
                // Extract complex rule components
                let all = simple_rules.get("all")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_object().map(|o| {
                        o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                    })).collect());

                let any = simple_rules.get("any")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_object().map(|o| {
                        o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                    })).collect());

                let not = simple_rules.get("not")
                    .and_then(|v| v.as_object())
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

                matches_complex(event, all.as_ref(), any.as_ref(), not.as_ref(), match_type)
            } else {
                matches_simple(event, simple_rules, match_type)
            }
        }
        Some(MatchRules::Complex { all, any, not }) => {
            matches_complex(event, all.as_ref(), any.as_ref(), not.as_ref(), match_type)
        }
    }
}

fn matches_simple(event: &Event, rules: &HashMap<String, Value>, match_type: &MatchType) -> bool {
    for (key, expected_value) in rules {
        // Support nested keys with dot notation (e.g., "tool.name")
        let actual_value = if key.contains('.') {
            event.get_nested_str(key).map(|s| Value::String(s.to_string()))
        } else {
            event.data.get(key).cloned()
        };

        match actual_value {
            Some(actual) => {
                if !values_match(&actual, expected_value, match_type) {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

fn matches_complex(
    event: &Event,
    all: Option<&Vec<HashMap<String, Value>>>,
    any: Option<&Vec<HashMap<String, Value>>>,
    not: Option<&HashMap<String, Value>>,
    match_type: &MatchType,
) -> bool {
    // Complex rules must have at least one field set
    if all.is_none() && any.is_none() && not.is_none() {
        return false;
    }

    // "all" - all conditions must match
    if let Some(all_rules) = all {
        for rule in all_rules {
            if !matches_simple(event, rule, match_type) {
                return false;
            }
        }
    }

    // "any" - at least one condition must match
    if let Some(any_rules) = any {
        let mut matched = false;
        for rule in any_rules {
            if matches_simple(event, rule, match_type) {
                matched = true;
                break;
            }
        }
        if !matched {
            return false;
        }
    }

    // "not" - condition must not match
    if let Some(not_rules) = not {
        if matches_simple(event, not_rules, match_type) {
            return false;
        }
    }

    true
}

fn values_match(actual: &Value, expected: &Value, match_type: &MatchType) -> bool {
    match (actual, expected) {
        // String matching - exact or regex
        (Value::String(a), Value::String(e)) => {
            match match_type {
                MatchType::Exact => a == e,
                MatchType::Regex => {
                    // Try to compile regex, fall back to exact match on error
                    Regex::new(e).map(|re| re.is_match(a)).unwrap_or(false)
                }
            }
        }
        (Value::Number(a), Value::Number(e)) => a == e,
        (Value::Bool(a), Value::Bool(e)) => a == e,

        // Array: check if expected array is subset of actual
        (Value::Array(a), Value::Array(e)) => e.iter().all(|ev| a.iter().any(|av| av == ev)),

        // Object: recursive matching
        (Value::Object(a), Value::Object(e)) => {
            e.iter().all(|(k, ev)| a.get(k).is_some_and(|av| values_match(av, ev, match_type)))
        }

        // Type mismatch
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_match() {
        let event = Event::from_json(r#"{"event_type": "success", "tool": "bash"}"#).unwrap();

        let mut rules = HashMap::new();
        rules.insert("event_type".to_string(), json!("success"));
        assert!(matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Exact));
    }

    #[test]
    fn test_simple_no_match() {
        let event = Event::from_json(r#"{"event_type": "error", "tool": "bash"}"#).unwrap();

        let mut rules = HashMap::new();
        rules.insert("event_type".to_string(), json!("success"));
        assert!(!matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Exact));
    }

    #[test]
    fn test_nested_match() {
        let event = Event::from_json(r#"{"tool": {"name": "bash", "status": "ok"}}"#).unwrap();

        let mut rules = HashMap::new();
        rules.insert("tool.name".to_string(), json!("bash"));
        assert!(matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Exact));
    }

    #[test]
    fn test_no_rules_matches_all() {
        let event = Event::from_json(r#"{"anything": "goes"}"#).unwrap();
        assert!(matches(&event, &None, &MatchType::Exact));
    }

    #[test]
    fn test_complex_all() {
        let event = Event::from_json(r#"{"type": "success", "tool": "bash"}"#).unwrap();

        let mut rule1 = HashMap::new();
        rule1.insert("type".to_string(), json!("success"));
        let mut rule2 = HashMap::new();
        rule2.insert("tool".to_string(), json!("bash"));

        let rules = MatchRules::Complex {
            all: Some(vec![rule1, rule2]),
            any: None,
            not: None,
        };

        assert!(matches(&event, &Some(rules), &MatchType::Exact));
    }

    #[test]
    fn test_complex_any() {
        let event = Event::from_json(r#"{"status": "ok"}"#).unwrap();

        let mut rule1 = HashMap::new();
        rule1.insert("status".to_string(), json!("ok"));
        let mut rule2 = HashMap::new();
        rule2.insert("status".to_string(), json!("error"));

        let rules = MatchRules::Complex {
            all: None,
            any: Some(vec![rule1, rule2]),
            not: None,
        };

        assert!(matches(&event, &Some(rules), &MatchType::Exact));
    }

    #[test]
    fn test_complex_not() {
        let event = Event::from_json(r#"{"status": "ok"}"#).unwrap();

        let mut not_rule = HashMap::new();
        not_rule.insert("status".to_string(), json!("error"));

        let rules = MatchRules::Complex {
            all: None,
            any: None,
            not: Some(not_rule),
        };

        assert!(matches(&event, &Some(rules), &MatchType::Exact));
    }
}

#[cfg(test)]
mod test_complex_any_hook_events {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_any_with_hook_event_name() {
        let event = Event::from_json(r#"{"hook_event_name": "Notification"}"#).unwrap();

        let mut rule1 = HashMap::new();
        rule1.insert("hook_event_name".to_string(), json!("Notification"));
        let mut rule2 = HashMap::new();
        rule2.insert("hook_event_name".to_string(), json!("Stop"));

        let rules = MatchRules::Complex {
            all: None,
            any: Some(vec![rule1, rule2]),
            not: None,
        };

        assert!(matches(&event, &Some(rules), &MatchType::Exact), "Should match Notification in any rules");
    }
}

#[cfg(test)]
mod test_misdeserialized_complex_rules {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_deserialized_complex_any_matches() {
        // When {"any": [...]} is deserialized as Simple instead of Complex,
        // the matcher should still handle it correctly
        let event = Event::from_json(r#"{"hook_event_name": "Notification"}"#).unwrap();

        let mut simple_map = HashMap::new();
        simple_map.insert("any".to_string(), json!([
            {"hook_event_name": "Notification"},
            {"hook_event_name": "Stop"}
        ]));

        assert!(matches(&event, &Some(MatchRules::Simple(simple_map)), &MatchType::Exact));
    }

    #[test]
    fn test_simple_deserialized_complex_any_no_match() {
        let event = Event::from_json(r#"{"hook_event_name": "PermissionRequest"}"#).unwrap();

        let mut simple_map = HashMap::new();
        simple_map.insert("any".to_string(), json!([
            {"hook_event_name": "Notification"},
            {"hook_event_name": "Stop"}
        ]));

        assert!(!matches(&event, &Some(MatchRules::Simple(simple_map)), &MatchType::Exact));
    }

    #[test]
    fn test_actual_simple_rule_not_affected() {
        // Make sure real simple rules still work
        let event = Event::from_json(r#"{"hook_event_name": "Notification"}"#).unwrap();

        let mut simple_map = HashMap::new();
        simple_map.insert("hook_event_name".to_string(), json!("Notification"));

        assert!(matches(&event, &Some(MatchRules::Simple(simple_map)), &MatchType::Exact));
    }

    #[test]
    fn test_complex_none_fields_no_match() {
        // Complex rules with all None fields should not match
        let event = Event::from_json(r#"{"hook_event_name": "Notification"}"#).unwrap();

        let rules = MatchRules::Complex {
            all: None,
            any: None,
            not: None,
        };

        assert!(!matches(&event, &Some(rules), &MatchType::Exact));
    }
}

#[cfg(test)]
mod test_regex_matching {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_regex_simple_pattern() {
        let event = Event::from_json(r#"{"message": "Claude needs your permission to use Write"}"#).unwrap();

        let mut rules = HashMap::new();
        rules.insert("message".to_string(), json!("Claude needs your permission.*"));
        assert!(matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Regex));
    }

    #[test]
    fn test_regex_no_match() {
        let event = Event::from_json(r#"{"message": "Something else"}"#).unwrap();

        let mut rules = HashMap::new();
        rules.insert("message".to_string(), json!("Claude needs your permission.*"));
        assert!(!matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Regex));
    }

    #[test]
    fn test_regex_with_complex_any() {
        let event = Event::from_json(r#"{"hook_event_name": "Notification", "message": "Claude needs your permission to use Write"}"#).unwrap();

        let mut rule1 = HashMap::new();
        rule1.insert("hook_event_name".to_string(), json!("PermissionRequest"));
        let mut rule2 = HashMap::new();
        rule2.insert("hook_event_name".to_string(), json!("Notification"));
        rule2.insert("message".to_string(), json!(".*permission.*"));

        let rules = MatchRules::Complex {
            all: None,
            any: Some(vec![rule1, rule2]),
            not: None,
        };

        assert!(matches(&event, &Some(rules), &MatchType::Regex));
    }

    #[test]
    fn test_exact_match_still_works() {
        let event = Event::from_json(r#"{"status": "success"}"#).unwrap();

        let mut rules = HashMap::new();
        rules.insert("status".to_string(), json!("success"));
        assert!(matches(&event, &Some(MatchRules::Simple(rules)), &MatchType::Exact));
    }
}
