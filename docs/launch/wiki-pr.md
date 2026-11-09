# Entries for the DS Pokémon Hacking wiki

Wiki repository: `ds-pokemon-hacking/ds-pokemon-hacking.github.io` (site: https://ds-pokemon-hacking.github.io/). Two entries, both in the neutral tone of the wiki. The wiki's README offers two routes: a pull request that follows https://ds-pokemon-hacking.github.io/contributing, or the issue template "Resource Submission" (title for the issue: `[Resource] Gen4 Studio, a text-based project editor for Gen IV`); the lines below work for both. It also suggests talking to the maintainers first in the #wiki-discussions channel of the Kingdom of DS Hacking Discord, which fits the launch order. The formats below were checked against the pages as they were on 2026-11-09; look at the current pages before opening the PR, because the tables change.

## Pull request

Title: `Add Gen4 Studio to the Mapping tools and the Getting Started tools`

Description:

```
Adds Gen4 Studio (https://github.com/Kaago/gen4-studio, MIT, macOS and Windows) to two tool lists: the "Tooling" table of the Generation IV mapping page and the "Recommended Tools" table of the Generation IV Getting Started page.

Gen4 Studio is an editor for Platinum, HeartGold and SoulSilver with a text-based project format (JSON, Git-friendly) and an MCP server for AI agents. It reads and writes the DSPRE project layout and shares no code with DSPRE. It contains no ROM data, and users supply their own ROM.

Thank you to the DSPRE team and to the authors of this wiki: Gen4 Studio builds on the documentation you wrote.
```

## 1. Mapping page

File: `docs/generation-iv/guides/mapping/mapping.md`, section "Tooling", the table `| Tool / Plugin Name | Notes |`. The existing rows are not in alphabetical order, so add this row at the end of the table, after the row for the painting tools (GIMP, Paint.net, Nitropaint):

```
| [Gen4 Studio](https://github.com/Kaago/gen4-studio) | Editor for Platinum, HeartGold and SoulSilver ROM hacks with a 2D and 3D world view and tile-based map painting that bakes to the `.nsbmd` model, `.bdhc` terrain and `.per` permission data. Imports PNG textures and OBJ or glTF models; projects use the DSPRE layout and keep game data as text files (JSON). |
```

## 2. Getting Started page

File: `docs/generation-iv/guides/00_getting_started/00_getting_started.md`, section "Recommended Tools", the HTML table `Objective | Tool(s) / Resource(s) | Tutorials`. The rows are ordered by objective, so add this row between "Pokémon Data Editing" and "ROM Tools":

```
    <tr>
        <td>Project Workflow (text-based project, Git)</td>
        <td>Edit worlds, scripts, texts and data in a text-based project that can be kept in Git, then build a ROM or an xdelta patch (macOS and Windows; Platinum, HeartGold and SoulSilver):<ul><li>[Gen4 Studio](https://github.com/Kaago/gen4-studio) (opens DSPRE project folders; includes an MCP server for AI agents)</li></ul></td>
        <td><ul><li>[Gen4 Studio Quickstart](https://github.com/Kaago/gen4-studio#quickstart)</li></ul></td>
    </tr>
```

If the maintainers prefer not to add a new objective, the same text fits in the "Tool(s) / Resource(s)" cell of the existing "Mapping" row as one more list item:

```
<li>[Gen4 Studio](https://github.com/Kaago/gen4-studio) (text-based projects with Git, 2D and 3D world view, opens DSPRE project folders)</li>
```
