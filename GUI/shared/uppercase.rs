//! Capitals for interface labels, shared by the Windows and Linux interfaces.
//! Greek all-capitals text drops the tonos; a tonos that kept two vowels
//! apart (ρολόι, άυλος) becomes a diaeresis on the second vowel instead.
fn without_tonos(c: char) -> Option<char> {
    Some(match c {
        'ά' | 'Ά' => 'Α', 'έ' | 'Έ' => 'Ε', 'ή' | 'Ή' => 'Η', 'ί' | 'Ί' => 'Ι',
        'ό' | 'Ό' => 'Ο', 'ύ' | 'Ύ' => 'Υ', 'ώ' | 'Ώ' => 'Ω',
        'ΐ' => 'Ϊ', 'ΰ' => 'Ϋ',
        _ => return None,
    })
}
fn greek(c: char) -> bool {
    ('\u{370}'..='\u{3ff}').contains(&c) || ('\u{1f00}'..='\u{1fff}').contains(&c)
}
pub fn upper(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // Capital vowel that carried a tonos, if it was the previous letter.
    let mut accented: Option<char> = None;
    for c in text.chars() {
        // Decomposed text: an acute/tonos combining mark after a Greek letter.
        if matches!(c, '\u{301}' | '\u{341}') && out.chars().last().is_some_and(greek) {
            accented = out.chars().last();
            continue;
        }
        if let Some(base) = without_tonos(c) {
            out.push(base);
            accented = Some(base);
            continue;
        }
        let capital: String = c.to_uppercase().collect();
        let separated = match (accented, capital.as_str()) {
            (Some('Α' | 'Ε' | 'Ο' | 'Υ'), "Ι") => Some('Ϊ'),
            (Some('Α' | 'Ε' | 'Ο' | 'Η'), "Υ") => Some('Ϋ'),
            _ => None,
        };
        match separated {
            Some(letter) => out.push(letter),
            None => out.push_str(&capital),
        }
        accented = None;
    }
    out
}
#[cfg(test)]
mod uppercase_tests {
    use super::upper;
    #[test]
    fn greek_capitals_drop_the_tonos() {
        for (text, capitals) in [
            ("Ρυθμίσεις", "ΡΥΘΜΙΣΕΙΣ"), ("Ζωντανά", "ΖΩΝΤΑΝΑ"), ("Ελληνικά", "ΕΛΛΗΝΙΚΑ"),
            ("Κανάλι", "ΚΑΝΑΛΙ"), ("Ήχος", "ΗΧΟΣ"), ("Ώρα", "ΩΡΑ"), ("Τίτλος", "ΤΙΤΛΟΣ"),
            ("προϊόν", "ΠΡΟΪΟΝ"), ("καΐκι", "ΚΑΪΚΙ"), ("Ευρώπη", "ΕΥΡΩΠΗ"), ("είναι", "ΕΙΝΑΙ"),
            ("Ρυθμι\u{301}σεις", "ΡΥΘΜΙΣΕΙΣ"),
        ] {
            assert_eq!(upper(text), capitals, "{text}");
        }
    }
    #[test]
    fn a_tonos_that_split_a_diphthong_becomes_a_diaeresis() {
        assert_eq!(upper("ρολόι"), "ΡΟΛΟΪ");
        assert_eq!(upper("άυλος"), "ΑΫΛΟΣ");
        assert_eq!(upper("τρόλεϊ"), "ΤΡΟΛΕΪ");
        assert_eq!(upper("αύριο"), "ΑΥΡΙΟ");
    }
    #[test]
    fn other_languages_are_unchanged() {
        for text in ["Settings", "Configurações", "Configuración", "straße", "LIVE", ""] {
            assert_eq!(upper(text), text.to_uppercase());
        }
    }
}
