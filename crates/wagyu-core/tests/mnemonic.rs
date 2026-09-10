//! Port of `tests/test_key_handling/test_key_derivation/test_mnemonic.py`.

mod common;

use common::{hex_to_bytes, read_vector, ABANDON};
use wagyu_core::mnemonic::{
    abbreviate_word, determine_mnemonic_language, language_from_name, language_name,
    mnemonic_from_entropy, parse_mnemonic, reconstruct_mnemonic, Language, ALL_LANGUAGES,
};
use wagyu_core::Error;

/// `(language, entropy, mnemonic, seed)` for every BIP-39 vector.
fn bip39_vectors() -> Vec<(Language, Vec<u8>, String, Vec<u8>)> {
    let vectors = read_vector("mnemonic.json");
    let mut out = Vec::new();
    for (lang_name, tests) in vectors.as_object().unwrap() {
        let language = language_from_name(lang_name).unwrap();
        for test in tests.as_array().unwrap() {
            let test = test.as_array().unwrap();
            out.push((
                language,
                hex_to_bytes(&test[0]),
                test[1].as_str().unwrap().to_string(),
                hex_to_bytes(&test[2]),
            ));
        }
    }
    assert_eq!(
        out.len(),
        9 * 24 - 8,
        "9 languages, Chinese lists have 20 vectors"
    );
    out
}

#[test]
fn test_bip39() {
    for (language, entropy, mnemonic, seed) in bip39_vectors() {
        assert_eq!(
            mnemonic_from_entropy(language, &entropy).unwrap().as_str(),
            mnemonic,
            "{}",
            language_name(language)
        );
        let parsed = parse_mnemonic(&mnemonic, Some(language)).unwrap();
        assert_eq!(
            parsed.seed("TREZOR").as_slice(),
            seed.as_slice(),
            "{mnemonic}"
        );
    }
}

#[test]
fn test_reconstruct_mnemonic() {
    for (language, _, mnemonic, _) in bip39_vectors() {
        let reconstructed =
            reconstruct_mnemonic(&mnemonic, None).unwrap_or_else(|e| panic!("{mnemonic}: {e}"));
        assert_eq!(reconstructed.as_str(), mnemonic);
        assert_eq!(
            parse_mnemonic(&mnemonic, None).unwrap().language(),
            language
        );
    }
}

#[test]
fn test_multi_lang_mnemonics() {
    let vectors = read_vector("multi_lang_mnemonic.json");
    let vectors = vectors.as_array().unwrap();
    assert_eq!(vectors.len(), 4);
    for mnemonic in vectors {
        let mnemonic = mnemonic.as_str().unwrap();
        match parse_mnemonic(mnemonic, None) {
            Err(Error::AmbiguousMnemonicLanguage(langs)) => {
                assert_eq!(
                    langs, "chinese_simplified, chinese_traditional",
                    "{mnemonic}"
                )
            }
            other => panic!(
                "{mnemonic}: expected ambiguity error, got {:?}",
                other.map(|m| m.phrase())
            ),
        }
        // Naming the language resolves it.
        assert!(parse_mnemonic(mnemonic, Some(Language::SimplifiedChinese)).is_ok());
        assert!(parse_mnemonic(mnemonic, Some(Language::TraditionalChinese)).is_ok());
    }
}

#[test]
fn test_reconstruct_abbreviated_mnemonic() {
    for (_, _, mnemonic, _) in bip39_vectors() {
        let abbreviated: Vec<String> = mnemonic.split(' ').map(abbreviate_word).collect();
        assert!(abbreviated.iter().all(|w| w.chars().count() <= 4));
        let abbreviated = abbreviated.join(" ");
        assert_eq!(
            reconstruct_mnemonic(&abbreviated, None)
                .unwrap_or_else(|e| panic!("{abbreviated}: {e}"))
                .as_str(),
            mnemonic
        );
    }
}

#[test]
fn test_determine_mnemonic_language() {
    let langs = determine_mnemonic_language("塞 香 廳 閉 勞 秦 可 貫 智 閣 慣 藝").unwrap();
    assert_eq!(
        langs,
        vec![Language::SimplifiedChinese, Language::TraditionalChinese]
    );
    let langs = determine_mnemonic_language(
        "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo when",
    )
    .unwrap();
    assert_eq!(langs, vec![Language::English]);
}

#[test]
fn test_determine_mnemonic_language_error() {
    assert!(matches!(
        parse_mnemonic("these are not words", None),
        Err(Error::InvalidMnemonic)
    ));
}

#[test]
fn every_language_round_trips_from_entropy() {
    let entropy = [0x7fu8; 32];
    for language in ALL_LANGUAGES {
        let mnemonic = mnemonic_from_entropy(language, &entropy).unwrap();
        assert_eq!(mnemonic.split_whitespace().count(), 24);
        let parsed = parse_mnemonic(&mnemonic, None)
            .unwrap_or_else(|e| panic!("{}: {e}", language_name(language)));
        assert_eq!(parsed.language(), language);
        assert_eq!(parsed.phrase().as_str(), mnemonic.as_str());
    }
}

#[test]
fn english_only_import_behaviour_is_a_superset() {
    // The old proxy validated with language='english'; naming English still works.
    assert!(parse_mnemonic(ABANDON, Some(Language::English)).is_ok());
    assert!(parse_mnemonic(ABANDON, None).is_ok());
}
