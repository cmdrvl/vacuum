pub fn guess_from_extension(extension: Option<&str>) -> Option<&'static str> {
    match extension.map(str::to_ascii_lowercase) {
        Some(value) => match value.as_str() {
            ".csv" => Some("text/csv"),
            ".tsv" => Some("text/tab-separated-values"),
            ".txt" => Some("text/plain"),
            ".json" => Some("application/json"),
            ".jsonl" => Some("application/x-jsonlines"),
            ".xml" => Some("application/xml"),
            ".pdf" => Some("application/pdf"),
            ".xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            ".xls" => Some("application/vnd.ms-excel"),
            ".parquet" => Some("application/vnd.apache.parquet"),
            ".zip" => Some("application/zip"),
            ".gz" => Some("application/gzip"),
            ".yaml" | ".yml" => Some("application/x-yaml"),
            _ => None,
        },
        None => None,
    }
}
