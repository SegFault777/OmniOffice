# OMD format, version 2

ZIP container, entries stored (uncompressed): `mimetype` (`application/vnd.omnioffice.omd+zip`, first),
`manifest.json` (`{"omd":2,...}`), `content.md`.

`content.md` is Markdown-compatible, one paragraph per line: `#`/`##`/`###` headings, `%` title, `>` quote,
4-space indent code, `**bold**`, `*italic*`, `<u>underline</u>`, and an optional alignment prefix
(`{c}` center, `{r}` right, `{j}` justify). Backslash escapes `\` and `*`. Version 1 files (no prefixes) still load.
