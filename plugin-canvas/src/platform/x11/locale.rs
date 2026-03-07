pub(super) fn get_locales() -> impl Iterator<Item=String> {
    let mut locales = Vec::new();

    if let Ok(languages) = std::env::var("LANGUAGE") && !languages.is_empty() {
        for locale in languages.split(":") {
            locales.push(locale.to_string());
        }
    }

    for env_variable in ["LC_ALL", "LANG"] {
        if let Ok(locale) = std::env::var(env_variable) && !locale.is_empty() {
            locales.push(locale.to_string());
        }
    }

    locales.into_iter()
}

