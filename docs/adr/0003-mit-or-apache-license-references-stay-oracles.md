# This project is MIT OR Apache-2.0; the vendored references keep their own licenses and stay test oracles

Every crate in this workspace, and the finder console's page, is licensed under the MIT license or the Apache License 2.0, at the user's option (`LICENSE-MIT`, `LICENSE-APACHE`). `Cargo.toml` has declared `license = "MIT OR Apache-2.0"` for the workspace since July 2026; this decision puts the license texts in the repository so the declaration means something, and so the project can be used, packaged and contributed to. The copies of tetra3, cedar-solve and cedar-detect vendored in `reference-solutions/` are not covered: they keep their own licenses (tetra3 and cedar-solve Apache-2.0, cedar-detect FSL-1.1-MIT), and, as the constitution already requires, they are oracles for development and tests, never runtime dependencies, and their source is never copied into `crates/`. The `star-detection` crate re-implements cedar-detect's algorithm and is held to cedar-detect's exact output by CI. It is this project's own code, written against that output, so cedar-detect's license does not reach it. The finder binary embeds third-party web packages and catalog data, whose notices `crates/finder/THIRD_PARTY_NOTICES.md` lists and `finder --licenses` prints. A contribution is licensed the same way unless its author says otherwise.

## Considered Options

- **Apache-2.0 only**, matching tetra3 and cedar-solve. Rejected: MIT OR Apache-2.0 is the Rust ecosystem's usual pair and what `Cargo.toml` already declares. It keeps Apache's patent grant for those who take that side, and MIT lets GPLv2-only projects use the code, which Apache-2.0 alone does not.
- **MIT only.** Rejected: simpler, but it drops Apache-2.0's explicit patent grant.
- **A copyleft license (GPL-3.0 or MPL-2.0).** Rejected for now: it would keep forks of the console open, but it narrows who can build the solver into a product or a finder of their own, and the goal is adoption. This is a choice the owner can revisit before the first release; afterwards it needs every contributor's agreement.
- **FSL-1.1-MIT for `star-detection` alone**, matching cedar-detect. Rejected: it is needed only if that crate carried cedar-detect's source, which the constitution forbids. One crate under a source-available license would also stop distributions and other projects from taking the workspace as a whole.

## Before the first release

- The owner confirms that no source from `reference-solutions/` was copied into `crates/`, `star-detection` above all. cedar-detect's own comments are quoted in places as the description of the behaviour reproduced (`crates/star-detection/src/detect.rs`), which is fine; its code would not be.
- Optional, and the owner's call: a courtesy note to Steven Rosenthal, cedar-detect's copyright holder, saying that the crate re-implements his detector under MIT OR Apache-2.0. His license invites contact about its terms, and the README already asks Cedar users for comparisons.
