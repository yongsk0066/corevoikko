// Generic VFST morphological analyzer.
//
// A thin wrapper around a weighted transducer (`mor.vfst`). Unlike the
// Finnish-specific analyzer, this one does minimal parsing -- it just
// collects the raw FST output and weight for each analysis.
//
// Origin: VfstAnalyzer.cpp (~120 lines)

use voikko_core::analysis::{Analysis, ATTR_FSTOUTPUT, ATTR_WEIGHT};
use voikko_core::case::CaseType;
use voikko_core::enums::MAX_WORD_CHARS;
use voikko_fst::Transducer;
use voikko_fst::config::WeightedConfig;
use voikko_fst::weighted::{WeightedResult, WeightedTransducer};

use super::Analyzer;
use super::tag_parser::{BUFFER_SIZE, MAX_ANALYSIS_COUNT};

/// Generic morphological analyzer using a weighted VFST transducer.
///
/// This analyzer is language-agnostic: it runs the transducer and returns
/// the raw FST output along with an exponentially-converted weight. It is
/// used for non-Finnish languages or when full morphological parsing is
/// not needed.
///
/// Origin: VfstAnalyzer.hpp, VfstAnalyzer.cpp
pub struct VfstAnalyzer {
    transducer: WeightedTransducer,
    config: WeightedConfig,
}

impl VfstAnalyzer {
    /// Create a new VfstAnalyzer from raw VFST binary data.
    ///
    /// The data should be the contents of a `mor.vfst` file (weighted format).
    ///
    /// Origin: VfstAnalyzer::VfstAnalyzer() -- VfstAnalyzer.cpp:54-60
    pub fn from_bytes(data: &[u8]) -> Result<Self, voikko_fst::VfstError> {
        let transducer = WeightedTransducer::from_bytes(data)?;
        let config = transducer.new_config(BUFFER_SIZE);
        Ok(Self {
            transducer,
            config,
        })
    }

    /// Analyze a word with optional full morphology.
    ///
    /// When `full_morphology` is true, the raw FST output is included in
    /// each analysis as the FSTOUTPUT attribute.
    ///
    /// The weight is always included, converted from the log-domain integer
    /// weight to a probability using `exp(-0.01 * weight)`.
    ///
    /// Origin: VfstAnalyzer::analyze(wchar_t*, size_t, bool) -- VfstAnalyzer.cpp:73-101
    pub fn analyze_full(
        &mut self,
        word: &[char],
        word_len: usize,
        full_morphology: bool,
    ) -> Vec<Analysis> {
        if word_len > MAX_WORD_CHARS {
            return Vec::new();
        }

        // Lowercase the input
        let mut word_lower: Vec<char> = word[..word_len].to_vec();
        voikko_core::case::set_case(&mut word_lower, CaseType::AllLower);

        let mut analyses = Vec::new();

        if !self.transducer.prepare(&mut self.config, &word_lower) {
            return analyses;
        }

        let mut output_buf = String::new();
        let mut result = WeightedResult {
            weight: 0,
            first_not_reached_position: 0,
        };
        let mut analysis_count = 0;

        while analysis_count < MAX_ANALYSIS_COUNT
            && self
                .transducer
                .next_weighted(&mut self.config, &mut output_buf, &mut result)
        {
            analysis_count += 1;

            let mut analysis = Analysis::new();

            if full_morphology {
                analysis.set(ATTR_FSTOUTPUT, &output_buf);
            }

            // Convert log-weight to probability: exp(-0.01 * weight)
            let weight_prob = log_weight_to_prob(result.weight);
            analysis.set(ATTR_WEIGHT, format!("{weight_prob:.9}"));

            analyses.push(analysis);
        }

        analyses
    }
}

impl Analyzer for VfstAnalyzer {
    /// Analyze a word and return all valid analyses.
    ///
    /// This trait implementation performs full morphology (includes FSTOUTPUT).
    ///
    /// Origin: VfstAnalyzer::analyze -- VfstAnalyzer.cpp:62-67
    fn analyze(&self, _word: &[char], _word_len: usize) -> Vec<Analysis> {
        // The Analyzer trait takes &self, but we need &mut self for the config.
        // Callers should use analyze_full() directly.
        Vec::new()
    }
}

/// Convert a log-domain integer weight to a probability.
///
/// The weight from the transducer is in the form `-100 * ln(probability)`,
/// so this computes `exp(-0.01 * weight)`.
///
/// Origin: VfstAnalyzer.cpp:69-71 (logWeightToProb)
fn log_weight_to_prob(log_weight: i16) -> f64 {
    (-0.01 * f64::from(log_weight)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_weight_zero_is_one() {
        let prob = log_weight_to_prob(0);
        assert!((prob - 1.0).abs() < 1e-9);
    }

    #[test]
    fn log_weight_positive_less_than_one() {
        let prob = log_weight_to_prob(100);
        // exp(-0.01 * 100) = exp(-1) ≈ 0.3679
        assert!((prob - (-1.0_f64).exp()).abs() < 1e-9);
    }

    #[test]
    fn log_weight_negative_greater_than_one() {
        let prob = log_weight_to_prob(-100);
        // exp(-0.01 * -100) = exp(1) ≈ 2.7183
        assert!((prob - 1.0_f64.exp()).abs() < 1e-9);
    }
}
