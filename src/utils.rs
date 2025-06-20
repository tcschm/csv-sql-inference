/// converts a string from snake_case or kebab-case to pascalcase.
pub fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .filter(|word| !word.is_empty())
        .map(|word| word[0..1].to_uppercase() + &word[1..].to_lowercase())
        .collect()
}

