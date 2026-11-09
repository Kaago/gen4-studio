# Awesome-list entries

One line per list, in the format of that list. One pull request per list and one entry per pull request. The formats and rules below were read from the lists' own README and contributing files on 2026-11-09; look at the neighbouring entries once more before opening each pull request, because the lists change quickly. Never fake popularity: where a list has a threshold, wait until the repository meets it.

## 1. awesome-mcp-servers

- List: https://github.com/punkpeye/awesome-mcp-servers (README.md)
- Section: "Gaming" (game data, game engines and services). Entries are meant to be alphabetical by `owner/repo` (the section is not perfectly sorted), so the line goes next to `justinscott12/modrinth-mcp` and before `kitao/pyxel-mcp`.
- Format: a bullet with `-`, the repository `owner/repo` as the link text pointing to its GitHub URL, then the icons for language, scope and operating systems, a plain hyphen and the description. The list's legend: 🦀 Rust codebase, 🏠 local service, 🍎 macOS, 🪟 Windows (no 🐧: there is no Linux build).
- Rules: one server per line, alphabetical order inside the category, concise description. The list is for servers with a public GitHub repository, which Gen4 Studio is once the repository is public. The server is the `studio mcp` subcommand of the desktop app.

```
- [Kaago/gen4-studio](https://github.com/Kaago/gen4-studio) 🦀 🏠 🍎 🪟 - Read and edit Pokémon Platinum, HeartGold and SoulSilver ROM-hack projects (maps, scripts, texts, data, build) through 90 curated tools, headless over stdio or inside the running desktop app, with read and write permissions per project.
```

Pull request title: `Add Kaago/gen4-studio`.

## 2. awesome-tauri: no entry possible

- List: https://github.com/tauri-apps/awesome-tauri
- Since pull request #876 ("Remove all applications", merged 2026-08-20), the README has only the sections Guides & Tutorials, Articles, Templates, Plugins and Integrations, and its contributing file says "Application submissions are not accepted anymore!". Gen4 Studio is an application, so there is no section for it and no line to propose. Do not open a pull request. Look again only if the list takes applications back.

## 3. awesome-rust

- List: https://github.com/rust-unofficial/awesome-rust (README.md)
- Section: "Applications" › "Games" (it already holds tools around games, for example save managers and mod downloaders). Entries are alphabetical by their displayed text, so the line goes between `HactarCE/Hyperspeedcube` and `lifthrasiir/angolmois-rust`.
- Format (from CONTRIBUTING.md): `* [ACCOUNT/REPO](https://github.com/ACCOUNT/REPO) [[CRATE](https://crates.io/crates/CRATE)] - DESCRIPTION`, without the crate part when there is no crates.io release, and the CI build badge after the description, separated by a space, with the branch in the image URL.
- Prerequisite: the entry needs more than 50 GitHub stars or more than 2,000 crates.io downloads (or an equivalent popularity measure that you state in the pull request). A new repository will not meet this on launch day. Open this pull request when the repository has passed 50 stars.

```
* [Kaago/gen4-studio](https://github.com/Kaago/gen4-studio) - Text-based, Git-friendly editor for Pokémon Platinum, HeartGold and SoulSilver ROM hacks, with a Tauri desktop app and an MCP server for AI agents [![build badge](https://github.com/Kaago/gen4-studio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Kaago/gen4-studio/actions)
```

## 4. awesome-romhacking

- List: https://github.com/btelwy/awesome-romhacking (README.md). A small list (about 50 stars) of ROM-hacking resources with a "Pokémon series" subsection under "Prominent Games" and a Nintendo DS subsection.
- Section: "Prominent Games" › "*Pokémon* series". The contributing file asks for new items at the bottom of a section, in the form `- [item name](https link) - Description beginning with capital, ending in period.`, always from a new branch. The README's own entries use `*` as the bullet and put the platform icons (🍎 macOS, 🪟 Windows, 🐧 Linux) at the end, so the line follows that.

```
* [Gen4 Studio](https://github.com/Kaago/gen4-studio) - A text-based, Git-friendly editor for Pokémon Platinum, HeartGold and SoulSilver ROM hacks, with an MCP interface for AI agents. 🍎🪟
```

## Lists looked at and left out

- `tobiasbueschel/awesome-pokemon` (the best-known Pokémon list): about Pokémon GO, game clones and data APIs, no ROM-hacking section, and not updated since November 2024.
- `romh-acking/Awesome-Rom-Hacking` (9 stars): organized by retro consoles and generic tools, with no place for a Gen IV project editor.
- The DS Pokémon Hacking wiki is not an awesome list; its entries are in [wiki-pr.md](wiki-pr.md).
