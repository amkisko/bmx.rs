use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SearchHit {
    pub(crate) name: String,
    pub(crate) url: String,
}

pub(crate) fn emit_search_hits(
    hits: &[SearchHit],
    print_url: bool,
    json: bool,
    include_url_column: bool,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(hits)?);
        return Ok(());
    }
    for hit in hits {
        if print_url {
            println!("{}", hit.url);
        } else if include_url_column {
            println!("{}\t{}", hit.name, hit.url);
        } else {
            println!("{}", hit.name);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SearchHit;

    #[test]
    fn search_hit_json_includes_name_and_url() {
        let hits = [SearchHit {
            name: "owner/repo".into(),
            url: "https://github.com/owner/repo.git".into(),
        }];
        let encoded = serde_json::to_string(&hits).unwrap();
        assert!(encoded.contains("owner/repo"));
        assert!(encoded.contains("https://github.com/owner/repo.git"));
    }
}
