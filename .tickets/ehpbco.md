---
status: closed
priority: 2
deps: []
attachments: []
created: 2026-11-10T07:32:45Z
updated: 2026-11-10T08:13:14Z
closed-at: 2026-11-10T08:13:14Z
---
# Decide whether aider.author-name should match a rendered ident

spec §5.2 renders an identity as 'Name <email>', but the default pattern aider.author-name is anchored \(aider\)\s*$, so it cannot match a real aider author read through git var; only a --author flag carrying the bare decoration matches. Fixing it is a registry change, so it needs agreement: either anchor the pattern differently, or render the author name without the email for the author target. Pinned by tests/identity_targets.rs::the_aider_name_pattern_does_not_match_a_rendered_ident.
