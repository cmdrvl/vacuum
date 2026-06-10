# vacuum Agent Ergonomics Scorecard - Pass 1

Target SHA: `46c13cea623d81c71da82bdbf577dfff8460f196`
Mode: `full`

## Summary

- Surfaces inventoried: 69
- Intent corpus: 116 entries, 116 useful hints, 0 useless errors, 0 silent failures
- Applied recommendations: 6
- Primary uplift areas: self-documentation, first-try success, intent inference, error pedagogy, safe alternatives

## Scores

| Dimension | Before | After | Evidence |
| --- | ---: | ---: | --- |
| Output parseability | 920 | 940 | Scan stdout remains JSONL; `--json` scan no-op tested in `tests/cli_smoke.rs`. |
| Exit-code clarity | 880 | 900 | Exit `0`/`2` preserved and advertised through `operator.json` and capabilities JSON. |
| Self-documentation | 620 | 880 | Added `vacuum --robot-triage`, `vacuum capabilities --json`, `vacuum robot-docs guide`. |
| Intent inference | 650 | 820 | `--json` accepted for scans; typo corpus reports 116/116 useful hints. |
| Error pedagogy | 610 | 830 | Root refusals now include `refusal.next_command`; unavailable fix mode names safe alternatives. |
| Dangerous-operation safety | 800 | 900 | `doctor --fix` remains unavailable and points to read-only alternatives. |

## Notes

Preflight caveat: the skill preflight reports missing `flock` on macOS. This pass ran single-agent; scripts used here either do not require `flock` or degrade when it is unavailable.
