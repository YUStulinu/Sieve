//! Romanian or English? Common words and Romanian diacritics decide; "en" when unsure.

#[rustfmt::skip]
const RO: &[&str] = &[
    "și", "si", "în", "este", "sunt", "pentru", "că", "ca", "cu", "pe", "la", "de", "din", "nu", "mai", "un", "o", "care", "acest",
    "această", "vă", "va", "mulțumesc", "multumesc", "bună", "buna", "ziua", "salut", "te", "rog", "am", "ai", "fost", "factura",
    "factură", "plată", "plata", "până", "pana", "dacă", "daca", "poți", "poti", "puteți", "puteti", "mâine", "maine", "azi",
];
const EN: &[&str] = &[
    "the", "and", "is", "are", "for", "that", "with", "on", "at", "of", "from", "not", "this", "you", "your", "we", "our", "be", "have",
    "has", "will", "would", "can", "could", "please", "thanks", "thank", "hi", "hello", "regards", "it", "to", "in", "a",
];

pub fn detect(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let mut ro = 0usize;
    let mut en = 0usize;
    for w in lower.split(|c: char| !c.is_alphabetic()).filter(|w| !w.is_empty()).take(400) {
        if RO.contains(&w) {
            ro += 1;
        }
        if EN.contains(&w) {
            en += 1;
        }
    }
    let diacritics = lower.chars().filter(|c| "ăâîșşțţ".contains(*c)).count();
    if ro * 10 + diacritics * 5 > en * 10 { "ro" } else { "en" }
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn tells_romanian_from_english() {
        assert_eq!(detect("Bună ziua, vă trimit factura pentru luna octombrie. Mulțumesc!"), "ro");
        assert_eq!(detect("Buna ziua, va trimit factura pentru luna octombrie. Multumesc!"), "ro");
        assert_eq!(detect("Hi Ana, could you send me the report by Friday? Thanks!"), "en");
        assert_eq!(detect(""), "en");
    }
}
