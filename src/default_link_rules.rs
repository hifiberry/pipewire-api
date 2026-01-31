use crate::linker::{LinkRule, LinkType, NodeIdentifier};

/// Get the default link rules for automatic connection
pub fn get_default_rules() -> Vec<LinkRule> {
    vec![
        LinkRule {
            source: NodeIdentifier {
                node_name: Some("^speakereq.x.\\.output$".to_string()),
                node_nick: None,
                object_path: None,
            },
            destination: NodeIdentifier {
                node_name: None,
                node_nick: None,
                object_path: Some("alsa:.*:sndrpihifiberry:.*:playback".to_string()),
            },
            link_type: LinkType::Link,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rules_exist() {
        let rules = get_default_rules();
        assert!(!rules.is_empty(), "Should have at least one default rule");
    }

    #[test]
    fn test_speakereq_rule() {
        let rules = get_default_rules();
        let speakereq_rule = &rules[0];
        
        assert_eq!(
            speakereq_rule.source.node_name.as_deref(),
            Some("^speakereq.x.\\.output$")
        );
        assert_eq!(
            speakereq_rule.destination.object_path.as_deref(),
            Some("alsa:.*:sndrpihifiberry:.*:playback")
        );
        assert!(matches!(speakereq_rule.link_type, LinkType::Link));
    }
}
