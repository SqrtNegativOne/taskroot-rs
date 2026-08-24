use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/ParsedSigils.generated.ts"
)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSigils {
    pub clean_title: String,
    pub properties: SigilProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/SigilProperties.generated.ts"
)]
#[serde(rename_all = "camelCase")]
pub struct SigilProperties {
    #[ts(optional)]
    pub priority: Option<i32>,
    pub tags: Vec<String>,
    #[ts(optional)]
    pub duration: Option<i32>,
    #[ts(optional)]
    pub day: Option<String>,
}

#[must_use]
pub fn parse_sigils(input: &str) -> ParsedSigils {
    let mut title_words = Vec::new();
    let mut properties = SigilProperties::default();

    for word in input.split_whitespace() {
        if let Some(tag) = word.strip_prefix('#') {
            if !tag.is_empty() {
                properties.tags.push(tag.to_string());
                continue;
            }
        }
        if let Some(pri) = word.strip_prefix('!') {
            if let Ok(p) = pri.parse::<i32>() {
                properties.priority = Some(p);
                continue;
            }
        }
        if let Some(est) = word.strip_prefix('^') {
            if let Some(m) = parse_duration(est) {
                properties.duration = Some(m);
                continue;
            }
        }
        if let Some(day) = word.strip_prefix('@') {
            if !day.is_empty() {
                properties.day = Some(day.to_string());
                continue;
            }
        }
        title_words.push(word);
    }

    ParsedSigils {
        clean_title: title_words.join(" "),
        properties,
    }
}

fn parse_duration(s: &str) -> Option<i32> {
    if s.is_empty() {
        return None;
    }
    if let Some(val) = s.strip_suffix('m') {
        return val.parse::<i32>().ok();
    }
    if let Some(val) = s.strip_suffix('h') {
        return val.parse::<i32>().ok().map(|h| h * 60);
    }
    s.parse::<i32>().ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn test_parse_sigils() {
        let input = "Buy milk !1 #groceries ^30m @today";
        let parsed = parse_sigils(input);
        assert_eq!(parsed.clean_title, "Buy milk");
        assert_eq!(parsed.properties.priority, Some(1));
        assert_eq!(parsed.properties.tags, vec!["groceries"]);
        assert_eq!(parsed.properties.duration, Some(30));
        assert_eq!(parsed.properties.day, Some("today".to_string()));

        let input2 = "Just a normal task";
        let parsed2 = parse_sigils(input2);
        assert_eq!(parsed2.clean_title, "Just a normal task");
        assert_eq!(parsed2.properties.priority, None);
        assert!(parsed2.properties.tags.is_empty());
        assert_eq!(parsed2.properties.duration, None);
        assert_eq!(parsed2.properties.day, None);

        let input3 = "^2h !notanumber # @";
        let parsed3 = parse_sigils(input3);
        assert_eq!(parsed3.clean_title, "!notanumber # @");
        assert_eq!(parsed3.properties.duration, Some(120));
    }
}
