# Security Policy

## Supported versions

Only the latest release of Gen4 Studio receives security fixes. If you run an older version, update first and check whether the problem is still there.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately with GitHub Private Vulnerability Reporting: open the [Security tab](https://github.com/Kaago/gen4-studio/security) of the repository, choose "Report a vulnerability" and describe the problem. Helpful details are the version, your platform, the steps to reproduce, and what an attacker could do with it.

Do not include a ROM, a save game or any file derived from one in a report. Describe the file instead (game, region, which structure), and the maintainer will reproduce it.

## What to expect

- You get an acknowledgement within 14 days.
- The maintainer assesses the report, keeps you informed, and agrees a date with you for the fix and the public advisory.
- A fix ships in a new release; the advisory credits you if you want to be named.

## Updates and signing

Gen4 Studio can check GitHub Releases for a new version: at startup only after the user agrees, otherwise only when asked (see the README, section "Updates"). Updates are signed with the project's own key (minisign, through the Tauri updater), and the app installs only a package whose signature matches the public key in `apps/studio/src-tauri/tauri.conf.json`. The private key and its password are never in this repository; they are kept by the maintainer and are available as secrets only to the bundle step of the release workflow.

If you suspect that the signing key is compromised, for example a signed package that did not come from a release of this repository, or a key that has leaked, report it as a vulnerability, privately and through the form described above, not in a public issue. A compromised key means a new key and a manual reinstall of Gen4 Studio, because installed versions trust only the old key.

## Scope

In scope are the Gen4 Studio app, its command line tool `g4k`, the MCP server (`studio mcp`), the update path (check, signature check and install) and the release scripts and workflows of this repository. Vulnerabilities in a dependency are best reported to that project; tell us as well if Gen4 Studio is affected in a way that needs a release.
