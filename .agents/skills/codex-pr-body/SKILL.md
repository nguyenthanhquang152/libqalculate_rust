---
name: codex-pr-body
description: Use when drafting or updating a pull request title and body for a Rust libqalculate port change, especially before publishing, review, or handoff.
---

# PR Body

Write PR metadata that helps reviewers understand why the Rust port changed and how compatibility was verified.

## Determine the PR

Use an explicit PR number or URL if provided. Otherwise infer it from the current branch with `gh pr view --json number,url,title,body`. If `gh` cannot infer the repository, add `--repo <owner>/<repo>` for the current project. Do not hardcode an upstream OpenAI repository.

Preserve important existing PR body content, especially images, links, task lists, issue references, and reviewer notes.

## Body Requirements

Include:

- Why the change is being made.
- What changed as the net effect of the PR.
- Upstream compatibility evidence, naming relevant `../libqalculate` source files, data files, or tests when applicable.
- Tests or manual checks that specifically prove the changed behavior.
- Review notes for intentional divergences, generated data, large fixtures, or staged follow-up work.

Omit:

- Absolute local paths outside the repository.
- Details about abandoned intermediate attempts.
- Routine checks that CI always runs unless they are the only available verification.
- Confidential codenames, private URLs, or unrelated implementation notes.

## Title

Use a concise imperative or noun-phrase title that names the behavior, not the implementation churn. Prefer compatibility terms a future maintainer can search, such as parser, number, formatter, unit, definition, fixture, or CLI.

## Stacked PRs

For stacked work, describe only the net change between the PR base and head. If the base is another PR branch, avoid re-describing changes that belong to the lower stack.
