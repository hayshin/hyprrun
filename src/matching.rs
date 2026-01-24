use crate::hyprland::Client;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct MatchCondition {
    pub field: MatchField,
    pub matcher: Matcher,
}

impl MatchCondition {
    pub fn new(field: MatchField, matcher: Matcher) -> Self {
        Self { field, matcher }
    }

    pub fn matches(&self, client: &Client) -> bool {
        self.field
            .value(client)
            .map(|value| self.matcher.matches(value))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MatchField {
    Class,
    InitialClass,
    Title,
    InitialTitle,
    Tag,
    XdgTag,
}

impl MatchField {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "class" | "c" => Some(Self::Class),
            "initial-class" | "initialClass" => Some(Self::InitialClass),
            "title" => Some(Self::Title),
            "initial-title" | "initialTitle" => Some(Self::InitialTitle),
            "tag" => Some(Self::Tag),
            "xdgtag" | "xdg-tag" | "xdgTag" => Some(Self::XdgTag),
            _ => None,
        }
    }

    fn value<'a>(&self, client: &'a Client) -> Option<&'a str> {
        match self {
            Self::Class => Some(client.class.as_str()),
            Self::InitialClass => client.initial_class.as_deref(),
            Self::Title => client.title.as_deref(),
            Self::InitialTitle => client.initial_title.as_deref(),
            Self::Tag => client.tag.as_deref(),
            Self::XdgTag => client.xdg_tag.as_deref(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Matcher {
    Equals(String),
    Contains(String),
    Prefix(String),
    Suffix(String),
    Regex(Regex),
}

impl Matcher {
    pub fn from_tokens(method: Option<&str>, pattern: &str) -> std::result::Result<Self, String> {
        let method = method.unwrap_or("contains");
        match method {
            "equals" | "eq" => Ok(Self::Equals(pattern.to_lowercase())),
            "contains" | "substr" => Ok(Self::Contains(pattern.to_lowercase())),
            "prefix" | "starts-with" | "startswith" => Ok(Self::Prefix(pattern.to_lowercase())),
            "suffix" | "ends-with" | "endswith" => Ok(Self::Suffix(pattern.to_lowercase())),
            "regex" | "re" => Regex::new(pattern)
                .map(Self::Regex)
                .map_err(|err| format!("Invalid regex `{pattern}`: {err}")),
            _ => Err(format!("Unsupported match method `{method}`")),
        }
    }

    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Equals(pattern) => value.to_lowercase() == *pattern,
            Self::Contains(pattern) => value.to_lowercase().contains(pattern),
            Self::Prefix(pattern) => value.to_lowercase().starts_with(pattern),
            Self::Suffix(pattern) => value.to_lowercase().ends_with(pattern),
            Self::Regex(regex) => regex.is_match(value),
        }
    }
}

pub fn parse_match_condition(value: &str) -> std::result::Result<MatchCondition, String> {
    let (selector, pattern) = value
        .split_once('=')
        .ok_or_else(|| "Expected matcher in the form field[:method]=pattern".to_string())?;

    if pattern.is_empty() {
        return Err("Matcher pattern cannot be empty".to_string());
    }

    let (field_token, method_token) = match selector.split_once(':') {
        Some((field, method)) => (field, Some(method)),
        None => (selector, None),
    };

    let field = MatchField::parse(field_token)
        .ok_or_else(|| format!("Unsupported match field `{field_token}`"))?;

    let matcher = Matcher::from_tokens(method_token, pattern)?;

    Ok(MatchCondition::new(field, matcher))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_client(
        class: &str,
        initial_class: Option<&str>,
        title: Option<&str>,
        initial_title: Option<&str>,
        tag: Option<&str>,
        xdg_tag: Option<&str>,
    ) -> Client {
        Client {
            class: class.to_owned(),
            address: "0x123".to_owned(),
            initial_class: initial_class.map(str::to_owned),
            title: title.map(str::to_owned),
            initial_title: initial_title.map(str::to_owned),
            tag: tag.map(str::to_owned),
            xdg_tag: xdg_tag.map(str::to_owned),
        }
    }

    fn matches(condition: &MatchCondition, client: &Client) -> bool {
        condition.matches(client)
    }

    #[test]
    fn matches_class_field() {
        let client = build_client("Firefox", None, None, None, None, None);
        let condition =
            MatchCondition::new(MatchField::Class, Matcher::Equals("firefox".to_string()));
        assert!(matches(&condition, &client));

        let failing =
            MatchCondition::new(MatchField::Class, Matcher::Equals("chromium".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_title_field() {
        let client = build_client("Firefox", None, Some("Docs - Firefox"), None, None, None);
        let condition =
            MatchCondition::new(MatchField::Title, Matcher::Contains("docs".to_string()));
        assert!(matches(&condition, &client));

        let failing =
            MatchCondition::new(MatchField::Title, Matcher::Contains("other".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_initial_class_field() {
        let client = build_client("Firefox", Some("firefox"), None, None, None, None);
        let condition = MatchCondition::new(
            MatchField::InitialClass,
            Matcher::Equals("firefox".to_string()),
        );
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(
            MatchField::InitialClass,
            Matcher::Equals("kitty".to_string()),
        );
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_initial_title_field() {
        let client = build_client(
            "Firefox",
            None,
            Some("Docs - Firefox"),
            Some("Welcome"),
            None,
            None,
        );
        let condition = MatchCondition::new(
            MatchField::InitialTitle,
            Matcher::Equals("welcome".to_string()),
        );
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(
            MatchField::InitialTitle,
            Matcher::Equals("other".to_string()),
        );
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_tag_field() {
        let client = build_client("Firefox", None, None, None, Some("work"), None);
        let condition = MatchCondition::new(MatchField::Tag, Matcher::Equals("work".to_string()));
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(MatchField::Tag, Matcher::Equals("play".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_xdgtag_field() {
        let client = build_client("Firefox", None, None, None, None, Some("browser"));
        let condition =
            MatchCondition::new(MatchField::XdgTag, Matcher::Equals("browser".to_string()));
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(MatchField::XdgTag, Matcher::Equals("video".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matcher_variants_behave_as_expected() {
        let client = build_client("Firebox", None, Some("Docs - Firebox"), None, None, None);

        let equals = Matcher::from_tokens(Some("equals"), "firebox").unwrap();
        assert!(equals.matches(&client.class));

        let contains = Matcher::from_tokens(Some("contains"), "docs").unwrap();
        assert!(contains.matches(client.title.as_deref().unwrap()));

        let prefix = Matcher::from_tokens(Some("prefix"), "docs").unwrap();
        assert!(prefix.matches(client.title.as_deref().unwrap()));

        let suffix = Matcher::from_tokens(Some("suffix"), "firebox").unwrap();
        assert!(suffix.matches(client.title.as_deref().unwrap()));

        let regex = Matcher::from_tokens(Some("regex"), "^Docs.*box$").unwrap();
        assert!(regex.matches(client.title.as_deref().unwrap()));
    }

    #[test]
    fn parse_match_condition_supports_aliases() {
        let initial_class = parse_match_condition("initialClass=kitty").unwrap();
        assert!(matches(
            &initial_class,
            &build_client("kitty", Some("kitty"), None, None, None, None)
        ));

        let initial_title = parse_match_condition("initial-title=welcome").unwrap();
        assert!(matches(
            &initial_title,
            &build_client("App", None, Some("App - now"), Some("Welcome"), None, None)
        ));

        let xdg_tag = parse_match_condition("xdg-tag=browser").unwrap();
        assert!(matches(
            &xdg_tag,
            &build_client("App", None, None, None, None, Some("browser"))
        ));
    }
}
