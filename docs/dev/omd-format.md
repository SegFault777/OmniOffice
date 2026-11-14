# Native formats

All three are ZIP containers with stored (uncompressed) entries: `mimetype`, `manifest.json`, one content file.

| Format | mimetype | content file |
|---|---|---|
| OMD v2 | `application/vnd.omnioffice.omd+zip` | `content.md`: Markdown-compatible, `{c}`/`{r}`/`{j}` alignment prefixes, `%` title, `<u>` underline |
| OMC v1 | `application/vnd.omnioffice.omc+zip` | `sheet.csv`: RFC 4180 CSV, cells starting with `=` are formulas |
| OMS v1 | `application/vnd.omnioffice.oms+zip` | `deck.md`: slides separated by `---`; `# title`, `- bullet`, `> speaker note` |

Readers ignore unknown entries.
