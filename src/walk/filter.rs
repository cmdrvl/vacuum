use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::record::builder::VacuumRecord;

pub fn apply_filters(
    records: Vec<VacuumRecord>,
    include: &[String],
    exclude: &[String],
) -> Vec<VacuumRecord> {
    let include_set = compile_globset(include);
    let exclude_set = compile_globset(exclude);

    records
        .into_iter()
        .filter(|record| {
            let candidate = normalize_relative(&record.relative_path);

            let include_match = if include.is_empty() {
                true
            } else {
                include_set
                    .as_ref()
                    .map(|set| set.is_match(&candidate))
                    .unwrap_or(false)
            };

            if !include_match {
                return false;
            }

            let exclude_match = if exclude.is_empty() {
                false
            } else {
                exclude_set
                    .as_ref()
                    .map(|set| set.is_match(&candidate))
                    .unwrap_or(false)
            };

            !exclude_match
        })
        .collect()
}

fn compile_globset(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    let mut has_pattern = false;

    for pattern in patterns {
        if let Ok(glob) = GlobBuilder::new(pattern).literal_separator(true).build() {
            builder.add(glob);
            has_pattern = true;
        }
    }

    if !has_pattern {
        return None;
    }

    builder.build().ok()
}

fn normalize_relative(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use crate::record::builder::VacuumRecord;

    use super::apply_filters;

    fn record(relative_path: &str) -> VacuumRecord {
        let mut record = VacuumRecord::empty();
        record.relative_path = relative_path.to_string();
        record
    }

    #[test]
    fn include_patterns_are_or_matched() {
        let records = vec![record("alpha.csv"), record("beta.txt"), record("gamma.pdf")];
        let include = vec!["*.csv".to_string(), "*.txt".to_string()];
        let filtered = apply_filters(records, &include, &[]);

        let mut kept = filtered
            .into_iter()
            .map(|record| record.relative_path)
            .collect::<Vec<_>>();
        kept.sort();

        assert_eq!(kept, vec!["alpha.csv", "beta.txt"]);
    }

    #[test]
    fn exclude_patterns_override_include_matches() {
        let records = vec![record("keep.csv"), record("drop.csv")];
        let include = vec!["*.csv".to_string()];
        let exclude = vec!["drop.*".to_string()];
        let filtered = apply_filters(records, &include, &exclude);

        let kept = filtered
            .into_iter()
            .map(|record| record.relative_path)
            .collect::<Vec<_>>();

        assert_eq!(kept, vec!["keep.csv"]);
    }

    #[test]
    fn matching_uses_forward_slash_normalized_relative_path() {
        let records = vec![record("nested\\inner\\file.csv")];
        let include = vec!["nested/**/*.csv".to_string()];
        let filtered = apply_filters(records, &include, &[]);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn star_pattern_matches_single_segment() {
        let records = vec![record("root.csv"), record("nested/root.csv")];
        let include = vec!["*.csv".to_string()];
        let filtered = apply_filters(records, &include, &[]);

        let kept = filtered
            .into_iter()
            .map(|record| record.relative_path)
            .collect::<Vec<_>>();
        assert_eq!(kept, vec!["root.csv"]);
    }

    #[test]
    fn double_star_pattern_matches_nested_segments() {
        let records = vec![record("root.csv"), record("nested/inner/root.csv")];
        let include = vec!["**/*.csv".to_string()];
        let filtered = apply_filters(records, &include, &[]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn question_mark_pattern_matches_single_character() {
        let records = vec![record("a1/file.csv"), record("a12/file.csv")];
        let include = vec!["a?/file.csv".to_string()];
        let filtered = apply_filters(records, &include, &[]);

        let kept = filtered
            .into_iter()
            .map(|record| record.relative_path)
            .collect::<Vec<_>>();
        assert_eq!(kept, vec!["a1/file.csv"]);
    }

    #[test]
    fn character_class_pattern_matches_expected_paths() {
        let records = vec![
            record("a1/file.csv"),
            record("b1/file.csv"),
            record("c1/file.csv"),
        ];
        let include = vec!["[ab]1/file.csv".to_string()];
        let filtered = apply_filters(records, &include, &[]);

        let mut kept = filtered
            .into_iter()
            .map(|record| record.relative_path)
            .collect::<Vec<_>>();
        kept.sort();

        assert_eq!(kept, vec!["a1/file.csv", "b1/file.csv"]);
    }
}
