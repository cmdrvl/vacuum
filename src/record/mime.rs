pub fn guess_from_extension(extension: Option<&str>) -> Option<&'static str> {
    let extension = extension?;

    if extension.eq_ignore_ascii_case(".csv") {
        Some("text/csv")
    } else if extension.eq_ignore_ascii_case(".tsv") {
        Some("text/tab-separated-values")
    } else if extension.eq_ignore_ascii_case(".txt") {
        Some("text/plain")
    } else if extension.eq_ignore_ascii_case(".json") {
        Some("application/json")
    } else if extension.eq_ignore_ascii_case(".jsonl") {
        Some("application/x-jsonlines")
    } else if extension.eq_ignore_ascii_case(".xml") {
        Some("application/xml")
    } else if extension.eq_ignore_ascii_case(".pdf") {
        Some("application/pdf")
    } else if extension.eq_ignore_ascii_case(".xlsx") {
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    } else if extension.eq_ignore_ascii_case(".xls") {
        Some("application/vnd.ms-excel")
    } else if extension.eq_ignore_ascii_case(".parquet") {
        Some("application/vnd.apache.parquet")
    } else if extension.eq_ignore_ascii_case(".zip") {
        Some("application/zip")
    } else if extension.eq_ignore_ascii_case(".gz") {
        Some("application/gzip")
    } else if extension.eq_ignore_ascii_case(".yaml") || extension.eq_ignore_ascii_case(".yml") {
        Some("application/x-yaml")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::guess_from_extension;

    #[test]
    fn maps_minimum_plan_extensions() {
        let cases = [
            (".csv", "text/csv"),
            (".tsv", "text/tab-separated-values"),
            (".txt", "text/plain"),
            (".json", "application/json"),
            (".jsonl", "application/x-jsonlines"),
            (".xml", "application/xml"),
            (".pdf", "application/pdf"),
            (
                ".xlsx",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (".xls", "application/vnd.ms-excel"),
            (".parquet", "application/vnd.apache.parquet"),
            (".zip", "application/zip"),
            (".gz", "application/gzip"),
            (".yaml", "application/x-yaml"),
            (".yml", "application/x-yaml"),
        ];

        for (extension, mime) in cases {
            assert_eq!(guess_from_extension(Some(extension)), Some(mime));
        }
    }

    #[test]
    fn unknown_extension_maps_to_none() {
        assert_eq!(guess_from_extension(Some(".unknown")), None);
        assert_eq!(guess_from_extension(None), None);
    }

    #[test]
    fn mapping_is_case_insensitive() {
        assert_eq!(
            guess_from_extension(Some(".JSONL")),
            Some("application/x-jsonlines")
        );
    }
}
