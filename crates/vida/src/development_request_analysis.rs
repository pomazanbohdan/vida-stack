pub(crate) fn coach_review_terms(normalized_request: &str) -> Vec<String> {
    contains_keywords(
        normalized_request,
        &[
            "acceptance criteria".to_string(),
            "against the spec".to_string(),
            "against spec".to_string(),
            "definition of done".to_string(),
            "implementation drift".to_string(),
            "implemented result".to_string(),
            "matches the spec".to_string(),
            "rework".to_string(),
            "spec compliance".to_string(),
            "spec conformance".to_string(),
        ],
    )
}

pub(crate) fn contains_keywords(request: &str, keywords: &[String]) -> Vec<String> {
    fn is_boundary(ch: Option<char>) -> bool {
        ch.map(|value| !value.is_alphanumeric() && value != '_')
            .unwrap_or(true)
    }

    fn bounded_match(request: &str, keyword: &str) -> bool {
        request.match_indices(keyword).any(|(start, _)| {
            let before = request[..start].chars().next_back();
            let after = request[start + keyword.len()..].chars().next();
            is_boundary(before) && is_boundary(after)
        })
    }

    keywords
        .iter()
        .filter(|keyword| {
            let keyword = keyword.as_str();
            if keyword.chars().count() <= 2 {
                return bounded_match(request, keyword);
            }
            if keyword.contains(' ') || keyword.contains('-') {
                return bounded_match(request, keyword);
            }
            if keyword
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return bounded_match(request, keyword);
            }
            request.contains(keyword)
        })
        .cloned()
        .collect()
}

pub(crate) fn feature_delivery_design_terms(request: &str) -> Vec<String> {
    let design_keywords = vec![
        "research".to_string(),
        "spec".to_string(),
        "specification".to_string(),
        "specifications".to_string(),
        "plan".to_string(),
        "planning".to_string(),
        "design".to_string(),
        "requirements".to_string(),
    ];
    let implementation_keywords = vec![
        "implement".to_string(),
        "implementation".to_string(),
        "write code".to_string(),
        "write the full code".to_string(),
        "full code".to_string(),
        "build".to_string(),
        "develop".to_string(),
    ];

    let design_matches = contains_keywords(request, &design_keywords);
    let implementation_matches = contains_keywords(request, &implementation_keywords);
    if design_matches.is_empty() || implementation_matches.is_empty() {
        return Vec::new();
    }

    let mut combined = Vec::new();
    for term in design_matches
        .into_iter()
        .chain(implementation_matches.into_iter())
    {
        if !combined.contains(&term) {
            combined.push(term);
        }
    }
    combined
}
