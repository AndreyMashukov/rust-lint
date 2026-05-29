# rust-lint

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Static analyzers for Rust that block the low-signal patterns which bloat a
codebase: narrating inline comments, tautological doc-comments,
defensive guards on values whose variant is already known, type-only test
assertions, runtime environment branching, ad-hoc `std::env::var` reads, banal
error wrappers, deferred `TODO`s, and `#[allow]` attributes used to silence the
linter instead of fixing the code. It also checks `Cargo.toml`: wildcard
versions and unsorted dependency tables (with `--fix`).

Rust counterpart of [go-lint](https://github.com/AndreyMashukov/go-lint) and
[rector-php-rules](https://github.com/AndreyMashukov/rector-php-rules). Same
philosophy, different syntax tree.

---

## Why

Low-effort code drifts the same way every time:

- Every line gets a narrating `//` comment that paraphrases the line itself.
- Functions get a doc-comment that restates the name (`/// Creates a new user`
  over `fn create_user`).
- Guards appear on values whose `Option`/`Result` variant is fixed at the call
  site (`if let Some(x) = Some(compute())`).
- Tests assert `x.is_some()` or `!v.is_empty()` — what the type system already
  guarantees — instead of pinning the actual value.
- Errors get wrapped with `.context("failed to read")` — strictly worse than
  returning the error, because it lengthens the chain without adding context.
- `if env == "prod"` branches sneak into production code, creating paths that
  run in only one environment and are exercised by no test.
- A `#[allow(...)]` appears next to anything the linter complained about.
- A `TODO` marks work that was decided against but left in the tree.

`rust-lint` **fails the build** when any of these appear. It is meant to be
wired into a pre-commit hook and CI as a hard gate, not advisory warnings — and
it holds its own source and tests to every rule it enforces.

---

## Install

```bash
cargo install --path .          # from a checkout
# or
cargo install --git https://github.com/AndreyMashukov/rust-lint
```

Requires a stable Rust toolchain (MSRV 1.74).

---

## Usage

```bash
rust-lint                       # lint . (all analyzers + .toml checks)
rust-lint src tests             # lint specific paths
rust-lint --no_inline_comment src   # run a single analyzer (skips .toml checks)
rust-lint --fix Cargo.toml      # sort dependency tables in place, then report
rust-lint --list                # list every analyzer
```

Exit codes: `0` clean, `1` findings reported, `2` usage error (e.g. unknown
`--analyzer`). With no `--analyzer` flags, all analyzers run plus the `.toml`
manifest checks; passing one or more `--analyzer` flags runs only those and
skips the manifest checks.

### As a pre-commit hook

```bash
#!/usr/bin/env bash
STAGED="$(git diff --cached --name-only --diff-filter=ACMR | grep -E '\.(rs|toml)$')"
[ -z "$STAGED" ] && exit 0
rust-lint $STAGED || exit 1
```

### GitHub Actions

```yaml
- uses: dtolnay/rust-toolchain@stable
- run: cargo install --git https://github.com/AndreyMashukov/rust-lint
- run: rust-lint src tests Cargo.toml
```

---

## Analyzers

| Analyzer | Catches |
|---|---|
| `no_inline_comment` | `//` and `/* */` comments inside function bodies |
| `no_allow_attr` | `#[allow(...)]` / `#[expect(...)]` lint suppression |
| `no_env_var` | `std::env::var` / `env::var` outside config modules |
| `no_env_branch` | `==`/`match` on `"prod"`/`"dev"`/`"test"` against an env selector |
| `no_panic_src` | `panic!`/`todo!`/`unimplemented!`/`unreachable!`, `.unwrap()`/`.expect()` in non-test code |
| `no_time_now` | `Instant/SystemTime/Utc/Local::now()` outside a clock module |
| `no_type_only_assert` | `assert!(x.is_some()/is_ok()/is_empty())` in tests |
| `no_db_mut_in_test` | raw `INSERT`/`UPDATE`/`DELETE`/… SQL in test code |
| `no_robot_doc` | doc-comments that restate the function name |
| `no_error_wrap_banality` | `.context`/`anyhow!`/`bail!` wrappers that add no context |
| `no_todo` | `TODO`/`FIXME`/`XXX`/`HACK` markers (any — owned or not) |
| `no_redundant_if` | `if c { return true } return false` and `if c { true } else { false }` |
| `no_dead_guard` | guards on a statically-known `Option`/`Result` variant |
| `no_silent_fallback` | `.unwrap_or` / `.unwrap_or_else` / `.unwrap_or_default` / `.ok_or` / `.ok_or_else` / `.map_or` / `.map_or_else` / `.get_or_insert` / `.get_or_insert_with` — silent defaults for missing values |

Plus the manifest checks `toml_unsorted_deps` and `toml_wildcard_dep`.

### `no_silent_fallback`

Flags every method call that turns an `Option`/`Result` into a default-on-missing value:
`.unwrap_or(default)`, `.unwrap_or_else(|| default)`, `.unwrap_or_default()`,
`.ok_or(err)`, `.ok_or_else(|| err)`, `.map_or(default, f)`, `.map_or_else(|| default, f)`,
`.get_or_insert(value)`, `.get_or_insert_with(|| value)`.

Test code is exempt — files under `tests/`, files matching `*_test.rs`, functions tagged
`#[test]`, and items inside `#[cfg(test)]` modules. Clippy's `allow-unwrap-in-tests` is the
established precedent for the same carve-out.

**Why.** A missing or `Err` value is information. Substituting a hidden default loses it.
Every silent default is a place where a misconfigured environment, a stale upstream payload,
or an AI-generated "safe" defaulter masks a real input problem. Propagate with `?`, branch
explicitly with `match`, or rework the upstream API so the value isn't `Option` in the first
place.

```rust
// BAD
let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080);
let title: String = row.title.unwrap_or_default();

// OK — propagate the error to the call site
fn load_port() -> Result<u16, ConfigError> {
    let raw = std::env::var("PORT").map_err(|_| ConfigError::Missing("PORT"))?;
    raw.parse().map_err(|_| ConfigError::Invalid("PORT"))
}

// OK — branch explicitly, refuse to silently substitute
let title = match row.title {
    Some(t) => t,
    None => return Err(DomainError::TitleRequired),
};
```

Sibling rules in the family: `no-silent-fallback` in
[`eslint-plugin-mess-detector`](https://github.com/AndreyMashukov/eslint-plugin-mess-detector)
(TS/JS — `??`, `??=`, `||` with literal RHS),
`NoSilentFallbackRector` in
[`rector-php-rules`](https://github.com/AndreyMashukov/rector-php-rules)
(PHP — `??`, `??=`, `isset(...) ? ... : ...`, `array_key_exists(...) ? ... : ...`, `?:`),
and `nosilentfallback` in
[`go-lint`](https://github.com/AndreyMashukov/go-lint)
(Go — `cmp.Or(x, <literal>)`, post-read string/numeric/nil/bool fallback inside `if`).

### no_inline_comment

Flags `//` and `/* */` comments inside function bodies. Doc comments,
`TODO`-family markers (owned by `no_todo`), and `rust-lint`/`want` directives
are exempt. The comment scanner is literal-aware: markers inside string, raw,
byte, and char literals are not comments, and nested block comments balance.

**Why.** Inline narration is the strongest sign of autopilot code. If a step needs prose to
explain, it needs a name that explains it. Comments rot; renamed functions do
not.

### no_allow_attr

Forbids `#[allow(...)]` and `#[expect(...)]`.

**Why.** Suppression masks the debt the linter exists to surface. Fix the issue
or drop the lint project-wide; do not paper over violations node by node.

### no_env_var

Flags `std::env::var` / `env::var` / `var_os` / `vars` outside modules named
`config`/`settings` or under a `/config/` path.

**Why.** Environment access scattered through business logic makes testing,
sandboxing, and documenting configuration impossible. Read it once; inject a
typed struct.

### no_env_branch

Flags `==`/`!=`/`match` against `"prod"`, `"dev"`, `"test"`, `"staging"`, … when
the other operand reads like an environment selector (`env`, `app_env`,
`config.environment`, `run_mode`, `profile`, …).

**Why.** Production code must behave identically in every environment. A branch
keyed on the env name creates a path that runs in only one of them — so it is
tested in none of the others.

### no_panic_src

Flags `panic!`, `todo!`, `unimplemented!`, `unreachable!`, and `.unwrap()` /
`.expect()` outside test code.

**Why.** Production code returns errors; it does not crash the process on a
value it failed to anticipate. Propagate with `?`.

### no_time_now

Flags `Instant::now`, `SystemTime::now`, `Utc::now`, `Local::now`,
`OffsetDateTime::now_utc`, … outside a `clock`/`time` module.

**Why.** Code that reads the clock directly cannot be tested deterministically.
Inject a `Clock` so tests can freeze time.

### no_type_only_assert

In test code, flags `assert!(x.is_some())`, `is_none`, `is_ok`, `is_err`, and
`is_empty` (including negated).

**Why.** These restate what the type system guarantees instead of pinning the
value. Assert it: `assert_eq!(x, Some(42))`.

### no_db_mut_in_test

Flags raw `INSERT`/`UPDATE`/`DELETE`/`TRUNCATE`/`DROP`/`ALTER` SQL string
literals in test code (matched as SQL, not prose).

**Why.** Tests that hand-write mutations set up state through a back door the
production code never uses, so they prove nothing about the real write path.

### no_robot_doc

Flags doc-comments (on public *and* private functions) that open by echoing the
function name and add nothing else: `/// Creates a new user` over
`fn create_user`.

**Why.** A doc that the signature already tells you costs a line and earns
nothing. Describe behavior, contracts, or edge cases — or omit it.

### no_error_wrap_banality

Flags `.context("...")`, `.with_context(|| "...")`, `anyhow!("...")`, and
`bail!("...")` whose message is a stock lead-in (`failed to`, `cannot`, …) plus
at most two bare words and an optional `{}` — i.e. no entity, id, or path.

**Why.** A wrapper that only prepends "failed to read" lengthens the error chain
without telling the reader anything the inner error didn't.

### no_todo

Flags every `TODO`/`FIXME`/`XXX`/`HACK` marker that opens a comment — an owner
or ticket does **not** redeem it.

**Why.** A deferred marker is work you decided not to do but left in the tree.
Implement it now, or track it in an issue and link it from real documentation —
do not leave the stub.

### no_redundant_if

Flags `if c { return true } return false` (and the inverse) and
`if c { true } else { false }`.

**Why.** The condition is the answer. `return c;` (or `!c`) says it directly.

### no_dead_guard

Flags guards whose outcome is fixed at the call site: `if let Some(x) = Some(v)`,
`if Ok(v).is_ok()`, `match None { … }`.

**Why.** Rust has no `nil`; its dead defensive guard is the check the compiler
already knows the answer to. It mimics caution while being unreachable in one
branch.

### toml_unsorted_deps / toml_wildcard_dep

`Cargo.toml` checks: dependency tables (`[dependencies]`, `[dev-dependencies]`,
`[build-dependencies]`, and `[target.*]` variants) must be sorted
alphabetically, and no dependency may be pinned to `"*"`. `rust-lint --fix`
sorts the tables in place (comments and formatting preserved); wildcards are
reported, never silently changed — only a human can choose the real version.

---

## Development

```bash
cargo test                                  # unit + integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- src tests Cargo.toml           # rust-lint linting itself
```

The last command is the dogfood gate: `rust-lint`'s own source and tests pass
every analyzer it ships, and CI enforces it.

## License

MIT — see [LICENSE](LICENSE).
