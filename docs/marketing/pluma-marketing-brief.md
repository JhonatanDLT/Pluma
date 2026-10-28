# Pluma — Marketing Brief (v1)

**One line:** Pluma is a local-first writing studio for novelists with long manuscripts — editor, story bible and optional local AI in one app, with nothing uploaded anywhere.

---

## 1. Positioning

**Category:** not "AI writing tool". Category = **the novelist's desktop** (Scrivener's job), with the privacy of a paper notebook and the structure of Obsidian.

**Positioning statement**
> For novelists writing 100k–500k+ words who are tired of stitching Word + Scrivener + Obsidian + a character spreadsheet together, **Pluma is the single local writing studio** that keeps the manuscript, the story bible and the AI on your own machine. Unlike cloud AI writing tools, **your draft is never uploaded, never trained on, and never metered by the word.**

**Hero copy**
- EN: *"Your novel never leaves your laptop."*
- ES (primary market): *"Tu novela no sale de tu portátil."*
- Sub: *"Outline, story bible, editor and export in one app. AI that runs on your machine — or not at all."*

**Message hierarchy (use in this order, everywhere)**
1. **Privacy / ownership** — local SQLite + Markdown, no account, no cloud, no training on your text.
2. **Built for long books** — 500k words, 40+ characters, 8 GB RAM, no lag (incremental analysis, per-chapter loading).
3. **The bible that fills itself** — characters, relationships, factions, places detected from the prose; you confirm, never fill forms.
4. **Zero lock-in** — import .docx/.pdf/.epub/.md; export .docx/.epub/.pdf/.md.
5. **AI is optional** — rules first, small local models second, your own API key third. Works fully offline.

**Positioning against**
| Alternative | Their weakness | Pluma's wedge |
|---|---|---|
| Scrivener | Dated UI, no entity analysis, Mac/Windows silo | Modern editor + auto story bible, same one-time ethos |
| Word / Google Docs | No structure, no bible, cloud | Structure derived from text, local by default |
| Obsidian/Notion stack | You build the system, no manuscript semantics | Works out of the box, no plugin archaeology |
| Sudowrite / Novelcrafter / ChatGPT | Subscription, uploads your manuscript, per-word anxiety | Flat price, local inference, unlimited words |
| Atticus / Vellum | Formatting only | Writing + bible + export in one place |

**Objections → answers (support, sales page, replies)**
- *"Is local AI good enough?"* → It's a linter and a brainstorming partner, not a ghostwriter. And it's optional.
- *"Will it handle my 400k-word saga?"* → Built for it; measurable perf benchmarks in the store listing.
- *"What if you disappear?"* → Markdown + SQLite on your disk. Nothing to migrate.
- *"Mac App Store sandbox?"* → Be honest about distribution limits (see §2).

**Spanish-first is a moat.** The UI, docs and support are native Spanish; virtually every competitor is English-first. Lead in ES, expand to EN with a dedicated launch, not a translated afterthought.

---

## 2. App Store Optimization

**Titles**
- macOS: `Pluma — Writing Studio` (23/30). ES: `Pluma — Estudio de novela` (26/30).
- Microsoft Store: `Pluma: Novel Writing Studio`.

**Subtitle / short description**
- Subtitle (30): `Offline novel writing studio` · ES: `Escribe tu novela sin nube`
- MS short description: *"Write, outline and track characters in one private, offline desktop studio. Import Word, export EPUB, PDF and DOCX. No account, no subscription required."*

**Categories**
- Mac App Store: **Productivity** (primary) · **Reference** (secondary).
- Microsoft Store: **Productivity** (primary) · **Books & reference** or **Education** (secondary — verify against current taxonomy).

**Keyword field (100 chars, no spaces, no repeats of title/subtitle, no competitor brands)**
```
novel,manuscript,outline,story bible,worldbuilding,scrivener alternative,epub,docx,offline,author,scene,character
```
Trim to ≤100 chars; rotate a second set for A/B: `saga,fantasy,screenplay,grammar,tts,markdown,local ai,privacy,selfpublishing,indie`

**ASO notes that actually decide approval/reach**
- **Sandbox risk:** downloading the ONNX Runtime binary on first run violates Mac App Store rules (only *data* may be downloaded post-install). Ship the MAS build with ORT bundled and models downloadable, or keep MAS to the no-AI core and sell local AI on the direct build.
- No "free" / "best" / "#1" in metadata; no other app names.
- Localize (not just translate) ES-MX, ES-ES, EN-GB, FR, DE, PT-BR — the store ranks per-locale and the field is nearly empty for Spanish writing apps.
- Screenshots 1–3 do the selling: (1) manuscript tree + bible, headline *"500,000 words. Zero lag."*; (2) character graph, *"Your bible writes itself."*; (3) Wi-Fi off icon, *"Works on a plane. Forever."*
- Ratings engine: in-app prompt after a completed export or a 30-day streak — the moment of pride.

---

## 3. Pricing

**Recommendation: one-time licence + paid major upgrades. No subscription at launch.**

Rationale: local inference = no marginal cost, so a subscription has no story; and the target buyer (privacy-conscious, Scrivener-trained, subscription-fatigued) reads subscriptions as rent on their own manuscript.

| Tier | Price | Contents |
|---|---|---|
| **Pluma Free** | $0 | Full editor, 1 project, export to .md/.txt. Enough to write a short story and fall in love. |
| **Pluma Studio** | **$59** (launch: **$39** founder price, first 500) | Unlimited projects, story bible, import/export all formats, versions, TTS, local models |
| **Pluma Studio + AI pack** | $79 | Bundled model installer, priority support |
| **Upgrade** | $25 on major versions | v1 buyers get v1.x forever |

- **No per-word, no token metering, no seat counting.** "Unlimited words" is a headline, not a footnote.
- **BYO API key** (OpenAI/Anthropic) costs you nothing — never resell tokens.
- Free tier is a distribution engine, not a hostage situation: one project, no watermark on export.
- Regional PPP pricing; 40% student/educator; free for NaNo-style participants during November (coded promo, not a public discount).
- 30-day refund, no questions — with local-only data, refunds are the cheapest trust signal you have.
- Distribution: direct (Paddle/Lemon Squeezy, 30% affiliate) → **Setapp** (strong fit for exactly this buyer) → MAS if sandbox issue solved → Microsoft Store later.

**Targets:** 3–5% free→paid, <5% refunds, 60% of revenue direct in year one.

---

## 4. Channels

**Priority 1 — where the buyer already is, in Spanish**
- X/Twitter `#ComunidadEscritora`, `#EscritoresDeTwitter` — the densest Spanish writing community; build in public from the account.
- Spanish YouTube writing channels (novel-structure, "cómo escribir una novela") — mid-tier sponsorships, 10k–150k subs.
- Wattpad ES / foro de escritores Discords; Literatura Fantástica forums.

**Priority 2 — English writer communities (post as a writer, never as an ad)**
- Reddit: r/writing, r/fantasywriters, r/selfpublish, r/Writeresearch, r/novelwriting. Rules are strict: share the workflow, answer craft questions, link only when asked. r/MacApps and r/LocalLLaMA for the technical angle; r/privacy for the "no cloud" angle.
- **November writing challenges** — NaNoWriMo the organization closed in 2025, but the ritual survives in r/nanowrimo, regional Discords and indie "WriMo" spinoffs. Own November: free licence for participants, daily word-count threads, a "Pluma Wrimo" template.
- Forums: AbsoluteWrite, Scribophile, Kboards, Codex, SFWA/SFWA-adjacent Discords, Inked Voices.
- Substack: sponsor 3–5 craft/newsletter writers in the 5k–50k range (fiction craft, indie publishing), plus your own build-in-public letter — the product story (Rust, local models, 500k-word perf) is genuinely interesting to this audience.
- Podcasts: The Self-Publishing Show, The Creative Penn, writing-craft shows; 60-second host-read spots convert better than display.
- YouTube: "writing setup / app tour" creators and Scrivener tutorial channels (comparison content ranks for years).

**Priority 3 — launch surfaces for technical credibility**
- Show HN + Product Hunt (angle: "a local-first Scrivener with an auto story bible, in Rust/Tauri").
- MacStories, 512 Pixels, Michael Tsai, AlternativeTo, Homebrew Cask.
- Conventions: 20BooksTo50K (indie authors, highest intent), local writers' guilds, Worldcon.

**Priority 4 — visual/aesthetic**
- TikTok/IG `#WritingTok`, `#BookTok`, dark-academia ("writing setup tour", rain + manuscript). Pluma's warm serif UI is the asset — ship a 4K screenshot pack and let creators use it.
- Pinterest boards for "writing desk" aesthetics → long-tail traffic to the landing page.

**Launch sequence (90 days)**
1. Wk 1–3: 50-novelist private beta from Spanish writing Discords; collect 10 recorded "I imported my 300k-word draft" moments.
2. Wk 4: Show HN + Product Hunt + r/MacApps, English landing page live.
3. Wk 5–8: November campaign, creator seeding (30 free licences, no strings), Setapp application.
4. Wk 9–12: MAS submission (sandbox-safe build), localized store listings, affiliate program open.

**Metrics that matter:** activation = manuscript imported within 24h; D30 retention of writers with ≥1,000 words logged; free→paid conversion; revenue per visitor on landing page; refund rate. Everything else is vanity.
