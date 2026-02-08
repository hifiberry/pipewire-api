use regex::Regex;

/// Match a string against a regex pattern
pub fn regex_match(pattern: &str, text: &str) -> bool {
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(text)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_match() {
        // Test basic regex patterns
        assert!(regex_match("^test.*", "test123"));
        assert!(regex_match(".*test$", "mytest"));
        assert!(regex_match(".*test.*", "myteststring"));
        assert!(regex_match("^test$", "test"));
        assert!(!regex_match("^test$", "test123"));
        assert!(regex_match("^node\\.", "node.input"));

        // Test single character patterns
        assert!(regex_match("^test.$", "test1"));
        assert!(regex_match("^test.$", "testa"));
        assert!(!regex_match("^test.$", "test12"));
        assert!(regex_match("^speakereq.x.\\.output$", "speakereq2x2.output"));
        assert!(regex_match("^speakereq.x.\\.output$", "speakereq4x4.output"));

        // Test complex patterns
        assert!(regex_match("alsa.*sndrpihifiberry.*playback", "alsa:acp:sndrpihifiberry:1:playback"));
        assert!(regex_match("alsa:.*:sndrpihifiberry:.*:playback", "alsa:acp:sndrpihifiberry:1:playback"));
        assert!(regex_match("^test..*", "test1234"));

        // Test escaped dot pattern
        assert!(regex_match("^effect_output\\.proc$", "effect_output.proc"));
    }
}
