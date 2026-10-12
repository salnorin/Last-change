# Future projects (separate, not part of this codebase)

## Child-protection detection
A detection capability for child sexual abuse material (CSAM) **must not** be built as a vigilante tool. It must be
developed **together with authorities and NGOs**, under legal guidance, using established hash-matching
programs (for example PhotoDNA through NCMEC, the INHOPE network, and in Germany jugendschutz.net / BKA). Nobody
should collect, store or "test with" such material themselves: possessing it is a crime.
Until then, Last-change relies on the hard-floor filter (`crates/core/src/filter.rs`), blocklists and reports.

## Reporting duties (Germany/EU), for node operators
- If you encounter CSAM or other serious crimes, **report them**: to the police/BKA (online reporting via the state
  police "Onlinewache"), to **jugendschutz.net**, or to the internet complaints hotlines of eco / FSM (part of INHOPE).
- Don't download, forward or screenshot such material. Block the entry locally (`lastchange block <id>`) and file a
  `report`.
- EU Digital Services Act (DSA) duties may apply to anyone *operating* a hosting service. Get legal advice.

## Planned governance (not implemented)
- One reputation-weighted vote per identity (capped or quadratic), with anonymous voting via STARK + nullifier.
- Scope is limited to moderation rules, filter additions and node removal. Votes can **only add** restrictions and
  can never permit hard-floor categories. The governance module will have no access to consensus/stamp parameters.
