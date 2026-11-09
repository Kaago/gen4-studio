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

## Scope

In scope are the Gen4 Studio app, its command line tool `g4k`, the MCP server (`studio mcp`) and the release scripts and workflows of this repository. Vulnerabilities in a dependency are best reported to that project; tell us as well if Gen4 Studio is affected in a way that needs a release.
