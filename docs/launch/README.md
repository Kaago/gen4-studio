# Launch texts for Gen4 Studio 1.0.0

Ready-to-post texts for the release of Gen4 Studio v1.0.0, all in English. The maintainer publishes them by hand, in the order below. Nothing in this folder is posted automatically.

Every text says what Gen4 Studio is, names macOS and Windows, the text project with Git, MCP and the DSPRE compatibility, thanks the DSPRE team and the DS hacking community, and links to the release page, the [README](../../README.md) and the [MCP guide](../en/mcp.md).

| File | For |
|---|---|
| [discord.md](discord.md) | Kingdom of DS ROM Hacking (KoDSH) Discord server, at most 1,200 characters, Discord Markdown |
| [wiki-pr.md](wiki-pr.md) | Two entries for the DS Pokémon Hacking wiki (Mapping tools and Getting Started) |
| [announcement.md](announcement.md) | Long form for PokéCommunity and the Project Pokémon file entry |
| [reddit.md](reddit.md) | Post for r/PokemonROMhacks |
| [show-hn.md](show-hn.md) | Show HN title, URL and the author's first comment |
| [awesome-lists.md](awesome-lists.md) | One line per awesome list, with the list's rules |

## Before the first post

All of these must be true before anything goes out, because every text links to the release page and the README.

- [ ] The release `v1.0.0` is published, not a draft, with the DMG, the Windows installer and `SHA256SUMS.txt` (two lines). Check: `gh release view v1.0.0 --repo Kaago/gen4-studio`. The [release guide](../en/release.md) describes the steps.
- [ ] The repository is public: `gh repo view Kaago/gen4-studio --json visibility` says `PUBLIC`. Discussions are switched on, so the follow-up below has a place to happen.
- [ ] The screenshots show in the README on github.com (they live in `docs/media/`) and contain nothing but what the app shows.
- [ ] Both packages were installed once from the release page, and the Gatekeeper and SmartScreen steps in the README match what you saw.
- [ ] The texts are still true: `bash scripts/check-links.sh docs/launch/*.md` prints `links: <n> ok`, `wc -m docs/launch/discord.md` is at most 1200, and `grep -niE 'based [o]n|fork [o]f' docs/launch` finds nothing (the DSPRE wording rule: inspired by, format knowledge, test oracle, project-compatible, shares no code).
- [ ] The links in the texts work on the public site: https://github.com/Kaago/gen4-studio/releases/latest, https://github.com/Kaago/gen4-studio#readme and https://github.com/Kaago/gen4-studio/blob/main/docs/en/mcp.md.
- [ ] Optional but friendly: tell the DSPRE maintainers about the launch before the first post, since every text thanks them.

## Publishing order

Post in this order and leave some time between the steps (a day or two between the community venues, so that you can answer everything). Stay reachable for the first few hours after each post.

1. **Discord, KoDSH first.** File: [discord.md](discord.md). The server invite published in the wiki's README is https://discord.gg/zAtqJDW2jC. Post in the channel for tools and releases (look at the channel list or ask the moderators; the wiki names #wiki-discussions for questions about the wiki). Paste the file as it is; the link without angle brackets shows the release preview.
   - [ ] Posted. Link to the message noted here: ________
2. **Wiki.** File: [wiki-pr.md](wiki-pr.md). Open the pull request, or the "Resource Submission" issue, in `ds-pokemon-hacking/ds-pokemon-hacking.github.io`, with the two entries. Read the current pages first, the tables change. Mention it in #wiki-discussions on the KoDSH Discord.
   - [ ] Opened. Link: ________
3. **PokéCommunity.** File: [announcement.md](announcement.md). Post in the ROM hacking tools area of the PokéCommunity forum. The forum uses BBCode, so turn the Markdown headings into bold lines, the bullets into lists and the links into URL tags.
   - [ ] Posted. Link: ________
4. **Project Pokémon file entry.** File: [announcement.md](announcement.md) (shortened to the description field if needed). Create a file entry in the closest category for ROM hacking tools, and use the GitHub release page as the download link rather than uploading the packages again, so the checksums and the release notes stay in one place.
   - [ ] Posted. Link: ________
5. **r/PokemonROMhacks.** File: [reddit.md](reddit.md). Read the sidebar rules and pick the flair for a tool release, replace the screenshots line with the link to the uploaded screenshots, then post. Do not ask for or post ROMs in the thread.
   - [ ] Posted. Link: ________
6. **Show HN.** File: [show-hn.md](show-hn.md). Submit the repository URL with the title from the file (see the note about the 80-character limit there), leave the text field empty and post the author comment right away. A weekday morning in US time is the usual choice. Do not ask friends to upvote.
   - [ ] Posted. Link: ________
7. **Awesome lists (pull requests).** File: [awesome-lists.md](awesome-lists.md). awesome-mcp-servers and awesome-romhacking can go out now. awesome-rust waits until the repository has more than 50 stars. awesome-tauri takes no applications any more, so there is nothing to submit there.
   - [ ] awesome-mcp-servers. Link: ________
   - [ ] awesome-romhacking. Link: ________
   - [ ] awesome-rust (when the threshold is met). Link: ________

## First week after the launch

Every day for the first week:

- [ ] Go through new issues and discussions (GitHub, plus the comment threads of the posts above). Answer the same day if you can, even if the answer is "I can reproduce this" or "not yet".
- [ ] Remove any ROM, save game or file derived from one that someone attaches, and explain kindly why (the contributing guide says so).
- [ ] Label what comes in and put every confirmed bug and every repeated question on one list for version 1.0.1. Windows reports come first, because the Windows build has not been checked by hand on a PC yet; Gatekeeper and SmartScreen questions point to gaps in the README.
- [ ] Thank people who report problems, and ask for the game, region and platform when a report lacks them.

At the end of the week:

- [ ] Update the section "Nach 1.0.0" in [docs/ROADMAP.md](../ROADMAP.md) (written in German) with what you learned: what people asked for, what broke, and what moved up or down.
- [ ] Decide the scope of 1.0.1 from the collected list. The version is changed only with `scripts/version.sh`.
- [ ] Open the pending awesome-list pull request(s) if a threshold has been reached since.
