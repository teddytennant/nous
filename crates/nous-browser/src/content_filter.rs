use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterAction {
    Block,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleCategory {
    Ads,
    Trackers,
    Malware,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub pattern: String,
    pub category: RuleCategory,
    pub action: FilterAction,
}

impl FilterRule {
    pub fn block(pattern: impl Into<String>, category: RuleCategory) -> Self {
        Self {
            pattern: pattern.into(),
            category,
            action: FilterAction::Block,
        }
    }

    pub fn allow(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            category: RuleCategory::Custom,
            action: FilterAction::Allow,
        }
    }

    pub fn matches(&self, url: &str) -> bool {
        let url_lower = url.to_lowercase();
        let pattern_lower = self.pattern.to_lowercase();

        if let Some(domain) = pattern_lower.strip_prefix("||") {
            // Domain anchor: matches domain or subdomain
            url_lower.contains(domain)
        } else if let Some(rest) = pattern_lower.strip_prefix('*')
            && let Some(inner) = rest.strip_suffix('*')
        {
            // Wildcard both sides
            url_lower.contains(inner)
        } else if let Some(prefix) = pattern_lower.strip_suffix('*') {
            // Prefix match
            url_lower.starts_with(prefix)
        } else if let Some(suffix) = pattern_lower.strip_prefix('*') {
            // Suffix match
            url_lower.ends_with(suffix)
        } else {
            // Substring match
            url_lower.contains(&pattern_lower)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilter {
    rules: Vec<FilterRule>,
    enabled: bool,
    blocked_count: u64,
}

impl ContentFilter {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            enabled: true,
            blocked_count: 0,
        }
    }

    pub fn with_default_rules() -> Self {
        let mut filter = Self::new();

        // Common ad networks
        let ad_domains = [
            "doubleclick.net",
            "googlesyndication.com",
            "googleadservices.com",
            "google-analytics.com",
            "adnxs.com",
            "adsrvr.org",
            "facebook.com/tr",
            "ads.twitter.com",
        ];
        for domain in ad_domains {
            filter.add_rule(FilterRule::block(format!("||{domain}"), RuleCategory::Ads));
        }

        // Common trackers
        let tracker_domains = [
            "analytics.google.com",
            "hotjar.com",
            "fullstory.com",
            "segment.io",
            "mixpanel.com",
            "amplitude.com",
            "sentry.io",
        ];
        for domain in tracker_domains {
            filter.add_rule(FilterRule::block(
                format!("||{domain}"),
                RuleCategory::Trackers,
            ));
        }

        filter
    }

    pub fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, index: usize) -> bool {
        if index < self.rules.len() {
            self.rules.remove(index);
            true
        } else {
            false
        }
    }

    pub fn should_block(&mut self, url: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check allow rules first (higher priority)
        for rule in &self.rules {
            if rule.action == FilterAction::Allow && rule.matches(url) {
                return false;
            }
        }

        // Check block rules
        for rule in &self.rules {
            if rule.action == FilterAction::Block && rule.matches(url) {
                self.blocked_count += 1;
                return true;
            }
        }

        false
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn blocked_count(&self) -> u64 {
        self.blocked_count
    }

    pub fn rules(&self) -> &[FilterRule] {
        &self.rules
    }

    pub fn rules_by_category(&self, category: RuleCategory) -> Vec<&FilterRule> {
        self.rules
            .iter()
            .filter(|r| r.category == category)
            .collect()
    }
}

impl Default for ContentFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_anchor_match() {
        let rule = FilterRule::block("||doubleclick.net", RuleCategory::Ads);
        assert!(rule.matches("https://ad.doubleclick.net/page"));
        assert!(rule.matches("http://doubleclick.net"));
        assert!(!rule.matches("https://example.com"));
    }

    #[test]
    fn wildcard_match() {
        let rule = FilterRule::block("*tracking*", RuleCategory::Trackers);
        assert!(rule.matches("https://example.com/tracking/pixel"));
        assert!(rule.matches("https://tracking.example.com"));
        assert!(!rule.matches("https://example.com"));
    }

    #[test]
    fn prefix_match() {
        let rule = FilterRule::block("https://ads.*", RuleCategory::Ads);
        assert!(rule.matches("https://ads.example.com"));
        assert!(!rule.matches("https://example.com/ads"));
    }

    #[test]
    fn suffix_match() {
        let rule = FilterRule::block("*.gif", RuleCategory::Ads);
        assert!(rule.matches("https://example.com/banner.gif"));
        assert!(!rule.matches("https://example.com/image.png"));
    }

    #[test]
    fn substring_match() {
        let rule = FilterRule::block("facebook.com/tr", RuleCategory::Trackers);
        assert!(rule.matches("https://facebook.com/tr?pixel=123"));
        assert!(!rule.matches("https://facebook.com/profile"));
    }

    #[test]
    fn case_insensitive() {
        let rule = FilterRule::block("||DoubleClick.NET", RuleCategory::Ads);
        assert!(rule.matches("https://doubleclick.net/ad"));
    }

    #[test]
    fn should_block_basic() {
        let mut filter = ContentFilter::new();
        filter.add_rule(FilterRule::block("||ads.example.com", RuleCategory::Ads));

        assert!(filter.should_block("https://ads.example.com/banner"));
        assert!(!filter.should_block("https://example.com/page"));
        assert_eq!(filter.blocked_count(), 1);
    }

    #[test]
    fn allow_overrides_block() {
        let mut filter = ContentFilter::new();
        filter.add_rule(FilterRule::block("||example.com", RuleCategory::Ads));
        filter.add_rule(FilterRule::allow("||example.com/good-page"));

        assert!(!filter.should_block("https://example.com/good-page"));
        assert!(filter.should_block("https://example.com/other"));
    }

    #[test]
    fn disabled_filter_allows_all() {
        let mut filter = ContentFilter::new();
        filter.add_rule(FilterRule::block("||ads.com", RuleCategory::Ads));
        filter.disable();

        assert!(!filter.should_block("https://ads.com/banner"));
        assert!(!filter.is_enabled());
    }

    #[test]
    fn default_rules() {
        let mut filter = ContentFilter::with_default_rules();
        assert!(filter.rule_count() > 0);
        assert!(filter.should_block("https://ad.doubleclick.net/page"));
        assert!(filter.should_block("https://analytics.google.com/collect"));
        assert!(!filter.should_block("https://example.com"));
    }

    #[test]
    fn rules_by_category() {
        let filter = ContentFilter::with_default_rules();
        let ad_rules = filter.rules_by_category(RuleCategory::Ads);
        let tracker_rules = filter.rules_by_category(RuleCategory::Trackers);

        assert!(!ad_rules.is_empty());
        assert!(!tracker_rules.is_empty());
    }

    #[test]
    fn remove_rule() {
        let mut filter = ContentFilter::new();
        filter.add_rule(FilterRule::block("test", RuleCategory::Custom));
        assert_eq!(filter.rule_count(), 1);
        assert!(filter.remove_rule(0));
        assert_eq!(filter.rule_count(), 0);
    }

    #[test]
    fn remove_rule_out_of_bounds() {
        let mut filter = ContentFilter::new();
        assert!(!filter.remove_rule(5));
    }

    #[test]
    fn filter_serializes() {
        let filter = ContentFilter::with_default_rules();
        let json = serde_json::to_string(&filter).unwrap();
        let restored: ContentFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.rule_count(), filter.rule_count());
    }

    #[test]
    fn blocked_count_increments() {
        let mut filter = ContentFilter::new();
        filter.add_rule(FilterRule::block("||ads.com", RuleCategory::Ads));

        filter.should_block("https://ads.com/1");
        filter.should_block("https://ads.com/2");
        filter.should_block("https://safe.com");

        assert_eq!(filter.blocked_count(), 2);
    }
}
