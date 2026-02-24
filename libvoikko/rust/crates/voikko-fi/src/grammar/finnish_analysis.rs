// Finnish morphological analysis to grammar token annotation
// Origin: grammar/FinnishAnalysis.hpp, FinnishAnalysis.cpp

use voikko_core::analysis::{
    ATTR_CLASS, ATTR_MOOD, ATTR_NEGATIVE, ATTR_PARTICIPLE, ATTR_PERSON,
    ATTR_POSSIBLE_GEOGRAPHICAL_NAME, ATTR_REQUIRE_FOLLOWING_VERB, ATTR_SIJAMUOTO, ATTR_STRUCTURE,
};
use voikko_core::enums::TokenType;

use crate::grammar::paragraph::{FollowingVerbType, GrammarToken, strip_soft_hyphens};
use crate::morphology::Analyzer;

// ---------------------------------------------------------------------------
// analyse_token
// Origin: FinnishAnalysis.cpp:53-192
// ---------------------------------------------------------------------------

/// Annotate a grammar token with morphological analysis flags.
///
/// Runs morphological analysis on the token's text and sets the boolean flags
/// used by the grammar rule engine. This is the Rust equivalent of
/// `FinnishAnalysis::analyseToken`.
///
/// The analyzer is called with soft hyphens stripped from the token text.
///
/// Origin: FinnishAnalysis.cpp:53-192
pub(crate) fn analyse_token(token: &mut GrammarToken, analyzer: &dyn Analyzer) {
    // Origin: FinnishAnalysis.cpp:54-65 — Initialize all flags.
    token.is_valid_word = false;
    token.possible_sentence_start = false;
    token.is_geographical_name_in_genitive = false;
    token.possible_geographical_name = false;
    token.possible_main_verb = false;
    token.possible_conjunction = false;

    // These three start as true and are set to false if ANY analysis
    // contradicts them (they represent "all analyses agree" semantics).
    token.is_main_verb = true;
    token.is_verb_negative = true;
    token.is_positive_verb = true;
    token.is_conjunction = true;

    token.require_following_verb = FollowingVerbType::None;
    token.verb_follower_type = FollowingVerbType::None;

    // Origin: FinnishAnalysis.cpp:66-71 — Non-word tokens get minimal flags.
    if token.token_type != TokenType::Word {
        token.first_letter_lcase = false;
        token.is_conjunction = false;
        token.is_verb_negative = false;
        return;
    }

    // Origin: FinnishAnalysis.cpp:73-78 — Strip soft hyphens and analyze.
    let word = strip_soft_hyphens(&token.text);
    let analyses = analyzer.analyze(&word, word.len());

    // Origin: FinnishAnalysis.cpp:81
    token.first_letter_lcase = true;
    let mut verb_follower_type_set = false;

    // Origin: FinnishAnalysis.cpp:83-184 — Iterate over all analyses.
    for (i, analysis) in analyses.iter().enumerate() {
        // Origin: FinnishAnalysis.cpp:84
        token.is_valid_word = true;

        let structure = analysis.get(ATTR_STRUCTURE).unwrap_or("");
        let wclass = analysis.get(ATTR_CLASS);
        let mood = analysis.get(ATTR_MOOD);
        let person = analysis.get(ATTR_PERSON);
        let negative = analysis.get(ATTR_NEGATIVE);
        let participle = analysis.get(ATTR_PARTICIPLE);
        let sijamuoto = analysis.get(ATTR_SIJAMUOTO);
        let possible_geo_name = analysis.get(ATTR_POSSIBLE_GEOGRAPHICAL_NAME);
        let require_following = analysis.get(ATTR_REQUIRE_FOLLOWING_VERB);

        // Origin: FinnishAnalysis.cpp:94-103 — first_letter_lcase / geographical name
        let structure_chars: Vec<char> = structure.chars().collect();
        if structure_chars.len() < 2 || (structure_chars[1] != 'p' && structure_chars[1] != 'q') {
            // Word may start with a capital letter anywhere.
            token.first_letter_lcase = false;

            // Check for geographical name in genitive case.
            // Origin: FinnishAnalysis.cpp:98-102
            if wclass == Some("paikannimi") && sijamuoto == Some("omanto") {
                token.is_geographical_name_in_genitive = true;
            }
        }

        // Origin: FinnishAnalysis.cpp:105-111 — conjunction detection
        if let Some(cls) = wclass {
            if cls == "sidesana"
                || (cls == "kieltosana"
                    && !token.text.is_empty()
                    && *token.text.last().unwrap() == '\u{00E4}')
            {
                // "enkä", "etkä", "eikä" = "ja en", ...
                token.possible_conjunction = true;
            } else {
                token.is_conjunction = false;
            }
        } else {
            token.is_conjunction = false;
        }

        // Origin: FinnishAnalysis.cpp:113-141 — verb classification
        match wclass {
            None => {
                // No word class: not a verb form we can classify.
                // Origin: FinnishAnalysis.cpp:113-118
                token.is_positive_verb = false;
                token.possible_main_verb = true;
                token.is_main_verb = false;
                token.is_verb_negative = false;
            }
            Some("kieltosana") => {
                // Negative word ("en", "et", "ei", etc.).
                // Origin: FinnishAnalysis.cpp:119-122
                token.is_positive_verb = false;
                token.is_main_verb = false;
            }
            Some("teonsana") => {
                // Verb.
                // Origin: FinnishAnalysis.cpp:123-136
                //
                // is_positive_verb: set to false if negative != "false", or
                // if mood is conditional and person is "3" (e.g. "en lukisi").
                if negative.is_none()
                    || negative != Some("false")
                    || ((mood.is_none() || mood == Some("conditional"))
                        && (person.is_none() || person == Some("3")))
                {
                    token.is_positive_verb = false;
                }

                // possible_main_verb: set if not an A/E-infinitive and not
                // a negative verb form.
                if (mood.is_none()
                    || (mood != Some("A-infinitive") && mood != Some("E-infinitive")))
                    && (negative.is_none() || negative != Some("true"))
                {
                    token.possible_main_verb = true;
                }

                // is_main_verb: only indicative mood verbs.
                if mood.is_none() || mood != Some("indicative") {
                    token.is_main_verb = false;
                }

                token.is_verb_negative = false;
            }
            Some(_) => {
                // Any other word class: not a verb.
                // Origin: FinnishAnalysis.cpp:137-141
                token.is_positive_verb = false;
                token.is_main_verb = false;
                token.is_verb_negative = false;
            }
        }

        // Origin: FinnishAnalysis.cpp:143-145 — possible geographical name
        if possible_geo_name == Some("true") {
            token.possible_geographical_name = true;
        }

        // Origin: FinnishAnalysis.cpp:146-161 — require_following_verb
        {
            let required_type = match require_following {
                Some("A-infinitive") => FollowingVerbType::AInfinitive,
                Some("MA-infinitive") => FollowingVerbType::MaInfinitive,
                _ => FollowingVerbType::None,
            };

            if required_type == FollowingVerbType::None || i == 0 {
                token.require_following_verb = required_type;
            } else if token.require_following_verb != required_type {
                token.require_following_verb = FollowingVerbType::None;
            }
        }

        // Origin: FinnishAnalysis.cpp:162-182 — verb_follower_type
        {
            let follower_type = match mood {
                Some("A-infinitive") => FollowingVerbType::AInfinitive,
                Some("MA-infinitive") => FollowingVerbType::MaInfinitive,
                _ => FollowingVerbType::None,
            };

            if follower_type != FollowingVerbType::None {
                if !verb_follower_type_set {
                    token.verb_follower_type = follower_type;
                    verb_follower_type_set = true;
                } else if token.verb_follower_type != follower_type {
                    token.verb_follower_type = FollowingVerbType::None;
                }
            } else if participle == Some("agent") && sijamuoto == Some("vajanto") {
                // Agent participle in abessive case: not a verb follower.
                // Origin: FinnishAnalysis.cpp:179-181
                token.verb_follower_type = FollowingVerbType::None;
            }
        }
    }

    // Origin: FinnishAnalysis.cpp:186-191 — If no valid analysis, clear verb flags.
    if !token.is_valid_word {
        token.is_positive_verb = false;
        token.is_conjunction = false;
        token.is_main_verb = false;
        token.is_verb_negative = false;
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use voikko_core::analysis::Analysis;

    // -- Mock analyzer ---------------------------------------------------------

    /// A mock analyzer that returns pre-configured analyses for specific words.
    struct MockAnalyzer {
        entries: Vec<(String, Vec<Analysis>)>,
    }

    impl MockAnalyzer {
        fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        /// Add an analysis result for a word.
        fn add(&mut self, word: &str, analyses: Vec<Analysis>) {
            self.entries.push((word.to_string(), analyses));
        }
    }

    impl Analyzer for MockAnalyzer {
        fn analyze(&self, word: &[char], _word_len: usize) -> Vec<Analysis> {
            let word_str: String = word.iter().collect();
            for (w, analyses) in &self.entries {
                if *w == word_str {
                    return analyses.clone();
                }
            }
            Vec::new()
        }
    }

    /// Helper to create an Analysis with key-value pairs.
    fn make_analysis(pairs: &[(&str, &str)]) -> Analysis {
        let mut a = Analysis::new();
        for &(k, v) in pairs {
            a.set(k, v);
        }
        a
    }

    /// Helper to create a word GrammarToken.
    fn word_token(text: &str) -> GrammarToken {
        GrammarToken::new(TokenType::Word, text.chars().collect(), 0)
    }

    // -- Non-word tokens -------------------------------------------------------

    #[test]
    fn non_word_token_gets_minimal_flags() {
        let analyzer = MockAnalyzer::new();
        let mut token = GrammarToken::new(TokenType::Punctuation, vec!['.'], 0);
        analyse_token(&mut token, &analyzer);

        assert!(!token.first_letter_lcase);
        assert!(!token.is_conjunction);
        assert!(!token.is_verb_negative);
        assert!(!token.is_valid_word);
    }

    #[test]
    fn whitespace_token_gets_minimal_flags() {
        let analyzer = MockAnalyzer::new();
        let mut token = GrammarToken::new(TokenType::Whitespace, vec![' '], 0);
        analyse_token(&mut token, &analyzer);

        assert!(!token.first_letter_lcase);
        assert!(!token.is_conjunction);
        assert!(!token.is_verb_negative);
    }

    // -- Unknown words ---------------------------------------------------------

    #[test]
    fn unknown_word_not_valid() {
        let analyzer = MockAnalyzer::new(); // No entries = not found
        let mut token = word_token("asdfgh");
        analyse_token(&mut token, &analyzer);

        assert!(!token.is_valid_word);
        assert!(!token.is_positive_verb);
        assert!(!token.is_conjunction);
        assert!(!token.is_main_verb);
        assert!(!token.is_verb_negative);
    }

    // -- Noun (nimisana) -------------------------------------------------------

    #[test]
    fn noun_analysis() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "koira",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ppppp"),
                (ATTR_CLASS, "nimisana"),
                (ATTR_SIJAMUOTO, "nimento"),
            ])],
        );

        let mut token = word_token("koira");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.first_letter_lcase); // structure starts with =p → lowercase
        assert!(!token.is_main_verb);
        assert!(!token.is_verb_negative);
        assert!(!token.is_positive_verb);
        assert!(!token.possible_main_verb);
    }

    // -- Proper noun (first letter uppercase) ----------------------------------

    #[test]
    fn proper_noun_not_first_letter_lcase() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "Helsinki",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ipppppppp"),
                (ATTR_CLASS, "paikannimi"),
                (ATTR_SIJAMUOTO, "nimento"),
            ])],
        );

        let mut token = word_token("Helsinki");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        // Structure starts with =i (uppercase), so first_letter_lcase = false.
        assert!(!token.first_letter_lcase);
    }

    // -- Geographical name in genitive -----------------------------------------

    #[test]
    fn geographical_name_in_genitive() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "Helsingin",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ipppppppppp"),
                (ATTR_CLASS, "paikannimi"),
                (ATTR_SIJAMUOTO, "omanto"),
            ])],
        );

        let mut token = word_token("Helsingin");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.is_geographical_name_in_genitive);
    }

    // -- Conjunction (sidesana) ------------------------------------------------

    #[test]
    fn conjunction_analysis() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "ja",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pp"),
                (ATTR_CLASS, "sidesana"),
            ])],
        );

        let mut token = word_token("ja");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.is_conjunction);
        assert!(token.possible_conjunction);
    }

    #[test]
    fn conjunction_with_other_analysis() {
        // Word has two analyses: one as conjunction, one as noun.
        // is_conjunction should be false (not all analyses agree),
        // but possible_conjunction should be true.
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "ja",
            vec![
                make_analysis(&[(ATTR_STRUCTURE, "=pp"), (ATTR_CLASS, "sidesana")]),
                make_analysis(&[(ATTR_STRUCTURE, "=pp"), (ATTR_CLASS, "nimisana")]),
            ],
        );

        let mut token = word_token("ja");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(!token.is_conjunction);
        assert!(token.possible_conjunction);
    }

    // -- Negative verb ending in ä (enkä, etkä, eikä) -------------------------

    #[test]
    fn negative_verb_ending_in_a_umlaut_is_possible_conjunction() {
        let mut analyzer = MockAnalyzer::new();
        let text = "eik\u{00E4}"; // "eikä"
        analyzer.add(
            text,
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pppp"),
                (ATTR_CLASS, "kieltosana"),
            ])],
        );

        let mut token = word_token(text);
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.possible_conjunction);
    }

    // -- Verb (teonsana) -------------------------------------------------------

    #[test]
    fn indicative_verb_is_main_verb() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "juoksi",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pppppp"),
                (ATTR_CLASS, "teonsana"),
                (ATTR_MOOD, "indicative"),
                (ATTR_NEGATIVE, "false"),
                (ATTR_PERSON, "1"),
            ])],
        );

        let mut token = word_token("juoksi");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.is_main_verb);
        assert!(token.possible_main_verb);
        assert!(token.is_positive_verb);
        assert!(!token.is_verb_negative);
    }

    #[test]
    fn a_infinitive_not_main_verb() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "juosta",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pppppp"),
                (ATTR_CLASS, "teonsana"),
                (ATTR_MOOD, "A-infinitive"),
            ])],
        );

        let mut token = word_token("juosta");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(!token.is_main_verb);
        assert!(!token.possible_main_verb); // A-infinitive is not possible main verb
        assert_eq!(token.verb_follower_type, FollowingVerbType::AInfinitive);
    }

    #[test]
    fn ma_infinitive_follower_type() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "juoksemaan",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pppppppppp"),
                (ATTR_CLASS, "teonsana"),
                (ATTR_MOOD, "MA-infinitive"),
            ])],
        );

        let mut token = word_token("juoksemaan");
        analyse_token(&mut token, &analyzer);

        assert_eq!(token.verb_follower_type, FollowingVerbType::MaInfinitive);
    }

    #[test]
    fn negative_verb_form() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "ei",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pp"),
                (ATTR_CLASS, "kieltosana"),
            ])],
        );

        let mut token = word_token("ei");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.is_verb_negative);
        assert!(!token.is_positive_verb);
        assert!(!token.is_main_verb);
    }

    // -- require_following_verb ------------------------------------------------

    #[test]
    fn require_following_verb_a_infinitive() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "haluan",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=pppppp"),
                (ATTR_CLASS, "teonsana"),
                (ATTR_MOOD, "indicative"),
                (ATTR_NEGATIVE, "false"),
                (ATTR_PERSON, "1"),
                (ATTR_REQUIRE_FOLLOWING_VERB, "A-infinitive"),
            ])],
        );

        let mut token = word_token("haluan");
        analyse_token(&mut token, &analyzer);

        assert_eq!(token.require_following_verb, FollowingVerbType::AInfinitive);
    }

    #[test]
    fn require_following_verb_ma_infinitive() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "alkaa",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ppppp"),
                (ATTR_CLASS, "teonsana"),
                (ATTR_MOOD, "indicative"),
                (ATTR_NEGATIVE, "false"),
                (ATTR_PERSON, "3"),
                (ATTR_REQUIRE_FOLLOWING_VERB, "MA-infinitive"),
            ])],
        );

        let mut token = word_token("alkaa");
        analyse_token(&mut token, &analyzer);

        // Person is "3" and mood is indicative, but negative is "false" and
        // mood is not conditional, so... actually the condition in the C++
        // sets is_positive_verb=false when the whole big condition is true.
        // For this test we focus on require_following_verb.
        assert_eq!(
            token.require_following_verb,
            FollowingVerbType::MaInfinitive
        );
    }

    #[test]
    fn conflicting_require_following_verb_resolves_to_none() {
        // Two analyses with different require_following_verb values.
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "verb",
            vec![
                make_analysis(&[
                    (ATTR_STRUCTURE, "=pppp"),
                    (ATTR_CLASS, "teonsana"),
                    (ATTR_MOOD, "indicative"),
                    (ATTR_REQUIRE_FOLLOWING_VERB, "A-infinitive"),
                ]),
                make_analysis(&[
                    (ATTR_STRUCTURE, "=pppp"),
                    (ATTR_CLASS, "teonsana"),
                    (ATTR_MOOD, "indicative"),
                    (ATTR_REQUIRE_FOLLOWING_VERB, "MA-infinitive"),
                ]),
            ],
        );

        let mut token = word_token("verb");
        analyse_token(&mut token, &analyzer);

        assert_eq!(token.require_following_verb, FollowingVerbType::None);
    }

    // -- verb_follower_type conflicts ------------------------------------------

    #[test]
    fn conflicting_verb_follower_type_resolves_to_none() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "verb",
            vec![
                make_analysis(&[
                    (ATTR_STRUCTURE, "=pppp"),
                    (ATTR_CLASS, "teonsana"),
                    (ATTR_MOOD, "A-infinitive"),
                ]),
                make_analysis(&[
                    (ATTR_STRUCTURE, "=pppp"),
                    (ATTR_CLASS, "teonsana"),
                    (ATTR_MOOD, "MA-infinitive"),
                ]),
            ],
        );

        let mut token = word_token("verb");
        analyse_token(&mut token, &analyzer);

        assert_eq!(token.verb_follower_type, FollowingVerbType::None);
    }

    // -- Agent participle in abessive ------------------------------------------

    #[test]
    fn agent_participle_abessive_not_verb_follower() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "lukematta",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ppppppppp"),
                (ATTR_CLASS, "teonsana"),
                (ATTR_PARTICIPLE, "agent"),
                (ATTR_SIJAMUOTO, "vajanto"),
            ])],
        );

        let mut token = word_token("lukematta");
        analyse_token(&mut token, &analyzer);

        assert_eq!(token.verb_follower_type, FollowingVerbType::None);
    }

    // -- possible_geographical_name --------------------------------------------

    #[test]
    fn possible_geographical_name_flag() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "Turku",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ipppp"),
                (ATTR_CLASS, "paikannimi"),
                (ATTR_POSSIBLE_GEOGRAPHICAL_NAME, "true"),
            ])],
        );

        let mut token = word_token("Turku");
        analyse_token(&mut token, &analyzer);

        assert!(token.possible_geographical_name);
    }

    // -- Soft hyphen stripping -------------------------------------------------

    #[test]
    fn soft_hyphens_stripped_before_analysis() {
        let mut analyzer = MockAnalyzer::new();
        // The analyzer receives the word without soft hyphens.
        analyzer.add(
            "koira",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ppppp"),
                (ATTR_CLASS, "nimisana"),
            ])],
        );

        // Token text includes a soft hyphen.
        let mut token = word_token("koi\u{00AD}ra");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
    }

    // -- Multiple analyses: "all agree" semantics ------------------------------

    #[test]
    fn all_analyses_must_agree_for_is_main_verb() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "word",
            vec![
                make_analysis(&[
                    (ATTR_STRUCTURE, "=pppp"),
                    (ATTR_CLASS, "teonsana"),
                    (ATTR_MOOD, "indicative"),
                ]),
                make_analysis(&[(ATTR_STRUCTURE, "=pppp"), (ATTR_CLASS, "nimisana")]),
            ],
        );

        let mut token = word_token("word");
        analyse_token(&mut token, &analyzer);

        // The noun analysis sets is_main_verb=false, so the token overall
        // should not be "definitely main verb".
        assert!(!token.is_main_verb);
    }

    #[test]
    fn no_class_sets_possible_main_verb() {
        // Analysis without CLASS attribute.
        let mut analyzer = MockAnalyzer::new();
        analyzer.add("thing", vec![make_analysis(&[(ATTR_STRUCTURE, "=ppppp")])]);

        let mut token = word_token("thing");
        analyse_token(&mut token, &analyzer);

        assert!(token.is_valid_word);
        assert!(token.possible_main_verb);
        assert!(!token.is_main_verb);
        assert!(!token.is_verb_negative);
        assert!(!token.is_positive_verb);
    }

    // -- first_letter_lcase with p/q structure ---------------------------------

    #[test]
    fn structure_p_means_lowercase() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "koira",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ppppp"),
                (ATTR_CLASS, "nimisana"),
            ])],
        );

        let mut token = word_token("koira");
        analyse_token(&mut token, &analyzer);

        assert!(token.first_letter_lcase);
    }

    #[test]
    fn structure_i_means_not_lowercase() {
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "Helsinki",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=ipppppppp"),
                (ATTR_CLASS, "paikannimi"),
            ])],
        );

        let mut token = word_token("Helsinki");
        analyse_token(&mut token, &analyzer);

        assert!(!token.first_letter_lcase);
    }

    #[test]
    fn structure_q_means_lowercase() {
        // STRUCTURE with 'q' at position 1 means lowercase abbreviation.
        let mut analyzer = MockAnalyzer::new();
        analyzer.add(
            "esim",
            vec![make_analysis(&[
                (ATTR_STRUCTURE, "=qqqq"),
                (ATTR_CLASS, "lyhenne"),
            ])],
        );

        let mut token = word_token("esim");
        analyse_token(&mut token, &analyzer);

        assert!(token.first_letter_lcase);
    }
}
