# Governance

## Overview

Axiom is an open-source project governed by its community. This document describes
how the project is managed, how decisions are made, and how contributors can grow
into maintainers.

## Roles

### Users

Anyone who uses Axiom. Users are encouraged to open issues, ask questions, and
participate in discussions.

### Contributors

Anyone who has submitted a pull request that was accepted. Contributors are listed
in [`CONTRIBUTORS.md`](CONTRIBUTORS.md) and the GitHub contributors graph.

### Reviewers

Experienced contributors with deep knowledge of a subsystem (e.g., `axiom-core`,
the REST API, a language binding). Reviewers:

- Triage issues and PRs in their area.
- Provide substantive code review.
- Cannot merge without a Maintainer's approval.

Becoming a Reviewer requires a track record of quality contributions and is
nominated by an existing Maintainer via a PR to this file.

### Maintainers

Maintainers have write access to the repository and are responsible for the overall
health of the project. Maintainers:

- Review and merge pull requests.
- Manage releases and the changelog.
- Set technical direction and roadmap priorities.
- Enforce the Code of Conduct.

The current Maintainers are listed in [`CODEOWNERS`](CODEOWNERS).

### Becoming a Maintainer

A Contributor may be nominated as a Maintainer by any existing Maintainer if they:

1. Have made substantial, high-quality contributions over at least 3 months.
2. Show good judgement in reviews and design discussions.
3. Are responsive to the community.

Nominations are opened as a PR to this file. Approval requires a supermajority
(⌈2/3⌉) of existing Maintainers. Once approved, the new Maintainer is added to
`CODEOWNERS` and granted write access.

### Removal

A Maintainer may step down voluntarily by opening a PR to remove themselves from
`CODEOWNERS`. A Maintainer may be removed by a supermajority vote of remaining
Maintainers for sustained unresponsiveness (> 6 months) or violation of the Code
of Conduct.

## Decision Making

Most decisions are made through lazy consensus: a proposed change (PR or issue
discussion) proceeds if no Maintainer objects within **5 business days**.

For **significant decisions** (new language bindings, breaking API changes, major
architectural shifts, CNCF lifecycle changes), a formal vote is required:

- Proposal posted as a GitHub Discussion with `proposal` label.
- Minimum 5-day comment period.
- Decision requires a supermajority (⌈2/3⌉) of active Maintainers.
- Result recorded in the Discussion thread.

## Technical Steering Committee (TSC)

If the project grows beyond 5 Maintainers, a TSC of up to 5 members may be
elected annually by the Maintainers to coordinate the roadmap. TSC decisions
follow the same supermajority rule.

## Meetings

- **Maintainer sync**: Bi-weekly video call (schedule posted in GitHub Discussions).
- **Community meeting**: Monthly open call; agenda posted 1 week in advance.
- All meeting notes are published in `docs/meeting-notes/`.

## Code of Conduct

All participants are expected to follow the
[Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
Violations are reported to `conduct@axiom-rules.io` and handled by the Maintainers.

## CNCF Relationship

Axiom is a [CNCF Sandbox](https://cncf.io) project. The CNCF TOC may provide
guidance on governance and is the final escalation path for unresolved conduct
violations.

## Amendments

This document may be amended by a supermajority vote of Maintainers, following
the same process as significant decisions above.
