---
name: code-review-context
description: Use when reviewing whether a Rust libqalculate port change provides enough bounded upstream, fixture, and local context for a reliable compatibility review.
---

# Context Review

Review whether the PR gives future reviewers the right evidence without dumping unbounded context.

## Required Context

- Link or name the upstream C++ files, fixtures, or data definitions used as the oracle.
- Explain any intentional divergence from upstream behavior.
- Identify copied, generated, or transformed data and how it can be refreshed.
- Name the Rust modules that own the behavior under review.

## Boundaries

- Flag any embedded artifact or copied snippet over 1k tokens for manual review.
- Reject unbounded generated context. Large data must have provenance, size bounds, and a regeneration path.
- Avoid absolute local paths except `../libqalculate`, which is the expected upstream checkout.
- Prefer file paths and focused excerpts over broad claims like "matches upstream".

## Findings

Report missing or excessive context as a review blocker when it prevents validating compatibility. Include the missing oracle, the local code affected, and the minimal context that should be added.
