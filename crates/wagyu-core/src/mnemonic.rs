//! BIP-39 mnemonic creation and validation.
//!
//! Ported from `ethstaker_deposit/key_handling/key_derivation/mnemonic.py`. The checksum and
//! seed derivation come from the `bip39` crate; this module only adds the deposit-cli
//! conveniences: 4-letter word abbreviations, language auto-detection and the "multiple valid
//! languages" error.

use std::sync::OnceLock;

pub use bip39::Language;
use bip39::Mnemonic;
use rand::rngs::OsRng;
use rand::TryRngCore;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

use crate::error::{Error, Result};

/// All supported languages, in the order the deposit-cli lists them. The order matters for the
/// wording of [`Error::AmbiguousMnemonicLanguage`].
pub const ALL_LANGUAGES: [Language; 10] = [
    Language::SimplifiedChinese,
    Language::TraditionalChinese,
    Language::Czech,
    Language::English,
    Language::French,
    Language::Italian,
    Language::Japanese,
    Language::Korean,
    Language::Portuguese,
    Language::Spanish,
];

/// The deposit-cli name of a language (`english`, `chinese_simplified`, ...).
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::SimplifiedChinese => "chinese_simplified",
        Language::TraditionalChinese => "chinese_traditional",
        Language::Czech => "czech",
        Language::English => "english",
        Language::French => "french",
        Language::Italian => "italian",
        Language::Japanese => "japanese",
        Language::Korean => "korean",
        Language::Portuguese => "portuguese",
        Language::Spanish => "spanish",
    }
}

/// Looks a language up by its deposit-cli name (case-insensitive).
pub fn language_from_name(name: &str) -> Result<Language> {
    let lower = name.trim().to_ascii_lowercase();
    ALL_LANGUAGES
        .iter()
        .copied()
        .find(|l| language_name(*l) == lower)
        .ok_or_else(|| Error::UnsupportedLanguage(name.to_string()))
}

/// A parsed mnemonic: the canonical full-word phrase and the language it was validated in.
pub struct ValidMnemonic {
    mnemonic: Mnemonic,
}

impl ValidMnemonic {
    pub fn language(&self) -> Language {
        self.mnemonic.language()
    }

    /// The full-word phrase, words separated by a single ASCII space.
    pub fn phrase(&self) -> Zeroizing<String> {
        Zeroizing::new(self.mnemonic.to_string())
    }

    /// BIP-39 seed (PBKDF2-HMAC-SHA512, 2048 rounds). `passphrase` is the optional "25th word";
    /// Wagyu always uses the empty string.
    pub fn seed(&self, passphrase: &str) -> Zeroizing<[u8; 64]> {
        Zeroizing::new(self.mnemonic.to_seed(passphrase))
    }
}

/// Creates a new 24-word mnemonic from 256 bits of OS entropy.
pub fn create_mnemonic(language: Language) -> Result<Zeroizing<String>> {
    let mut entropy = Zeroizing::new([0u8; 32]);
    OsRng
        .try_fill_bytes(entropy.as_mut())
        .map_err(|e| Error::Rng(e.to_string()))?;
    mnemonic_from_entropy(language, entropy.as_ref())
}

/// Deterministic variant of [`create_mnemonic`], used by the BIP-39 test vectors.
pub fn mnemonic_from_entropy(language: Language, entropy: &[u8]) -> Result<Zeroizing<String>> {
    let mnemonic =
        Mnemonic::from_entropy_in(language, entropy).map_err(|_| Error::InvalidMnemonic)?;
    Ok(Zeroizing::new(mnemonic.to_string()))
}

/// First four characters of the lowercased, NFKC-normalised word. BIP-39 word lists guarantee
/// that four characters identify a word uniquely within a language.
pub fn abbreviate_word(word: &str) -> String {
    word.to_lowercase().nfkc().take(4).collect()
}

/// Every language that contains at least one of the (abbreviated) words, in [`ALL_LANGUAGES`]
/// order. This mirrors `determine_mnemonic_language` in the Python code, which returns the union
/// over all words rather than the intersection.
pub fn determine_mnemonic_language(mnemonic: &str) -> Result<Vec<Language>> {
    let words: Vec<String> = mnemonic.split_whitespace().map(abbreviate_word).collect();
    let languages: Vec<Language> = ALL_LANGUAGES
        .iter()
        .copied()
        .filter(|lang| {
            let list = abbreviated_word_list(*lang);
            words.iter().any(|w| list.contains(w))
        })
        .collect();
    if languages.is_empty() {
        return Err(Error::InvalidMnemonic);
    }
    Ok(languages)
}

/// Validates a mnemonic and returns its canonical form.
///
/// - Words may be abbreviated to their first four characters.
/// - Words are matched case-insensitively and after NFKC normalisation.
/// - Any Unicode whitespace separates words (Japanese mnemonics use U+3000).
/// - Without `language`, every language is tried; a mnemonic that validates in more than one
///   language is rejected with [`Error::AmbiguousMnemonicLanguage`].
pub fn parse_mnemonic(input: &str, language: Option<Language>) -> Result<ValidMnemonic> {
    let words: Vec<String> = input.split_whitespace().map(abbreviate_word).collect();
    if !matches!(words.len(), 12 | 15 | 18 | 21 | 24) {
        return Err(Error::InvalidMnemonic);
    }

    let candidates = match language {
        Some(l) => vec![l],
        None => determine_mnemonic_language(input)?,
    };

    let mut valid: Vec<Mnemonic> = Vec::new();
    for lang in candidates {
        let list = lang.word_list();
        let abbreviated = abbreviated_word_list(lang);
        let mut full_words = Vec::with_capacity(words.len());
        for word in &words {
            match abbreviated.iter().position(|a| a == word) {
                Some(i) => full_words.push(list[i]),
                None => break,
            }
        }
        if full_words.len() != words.len() {
            continue;
        }
        let phrase = full_words.join(" ");
        if let Ok(m) = Mnemonic::parse_in_normalized(lang, &phrase) {
            valid.push(m);
        }
    }

    match valid.len() {
        0 => Err(Error::InvalidMnemonic),
        1 => Ok(ValidMnemonic {
            mnemonic: valid.pop().expect("one element"),
        }),
        _ => {
            let names: Vec<&str> = valid.iter().map(|m| language_name(m.language())).collect();
            Err(Error::AmbiguousMnemonicLanguage(names.join(", ")))
        }
    }
}

/// Validates a mnemonic (see [`parse_mnemonic`]) and returns the full-word phrase.
pub fn reconstruct_mnemonic(input: &str, language: Option<Language>) -> Result<Zeroizing<String>> {
    Ok(parse_mnemonic(input, language)?.phrase())
}

fn abbreviated_word_list(language: Language) -> &'static [String] {
    static LISTS: OnceLock<Vec<(Language, Vec<String>)>> = OnceLock::new();
    let lists = LISTS.get_or_init(|| {
        ALL_LANGUAGES
            .iter()
            .map(|lang| {
                let abbreviated = lang
                    .word_list()
                    .iter()
                    .map(|w| abbreviate_word(w))
                    .collect();
                (*lang, abbreviated)
            })
            .collect()
    });
    &lists
        .iter()
        .find(|(l, _)| *l == language)
        .expect("all languages are pre-computed")
        .1
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABANDON: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn create_gives_24_english_words() {
        let m = create_mnemonic(Language::English).unwrap();
        assert_eq!(m.split(' ').count(), 24);
        assert!(parse_mnemonic(&m, None).is_ok());
    }

    #[test]
    fn abbreviated_and_mixed_case_words_are_expanded() {
        let abbreviated = "ABAN aban aban aban aban aban aban aban aban aban aban abou";
        assert_eq!(
            reconstruct_mnemonic(abbreviated, None).unwrap().as_str(),
            ABANDON
        );
    }

    #[test]
    fn bad_checksum_and_bad_length_are_rejected() {
        let bad = "abandon ".repeat(12);
        assert!(matches!(
            parse_mnemonic(&bad, None),
            Err(Error::InvalidMnemonic)
        ));
        let short = "abandon ".repeat(11);
        assert!(matches!(
            parse_mnemonic(&short, None),
            Err(Error::InvalidMnemonic)
        ));
        assert!(matches!(
            parse_mnemonic("these are not words", None),
            Err(Error::InvalidMnemonic)
        ));
    }

    #[test]
    fn explicit_wrong_language_is_rejected() {
        assert!(parse_mnemonic(ABANDON, Some(Language::French)).is_err());
        assert!(parse_mnemonic(ABANDON, Some(Language::English)).is_ok());
    }

    #[test]
    fn language_names_round_trip() {
        for l in ALL_LANGUAGES {
            assert_eq!(language_from_name(language_name(l)).unwrap(), l);
        }
        assert!(language_from_name("klingon").is_err());
    }
}
