# Native formats

All three are ZIP containers with stored (uncompressed) entries: `mimetype`, `manifest.json`, one content file.

| Format | mimetype | content file |
|---|---|---|
| OMD v3 | `application/vnd.omnioffice.omd+zip` | `content.md`: Markdown-compatible, `{c}`/`{r}`/`{j}` alignment prefixes, `%` title, `<u>` underline, `~~strike~~`, `<span s=14 c=C00000>` size/colour, pipe tables, `![alt](media/img1.png)` pictures stored as `media/*` entries |
| OMC v2 | `application/vnd.omnioffice.omc+zip` | `sheet.csv` (RFC 4180, `=` starts a formula), `formats.csv` (`row,col,bold,align,numfmt,fill,color`), `charts.csv` (`kind,r1,c1,r2,c2,atRow,atCol,title`) |
| OMS v3 | `application/vnd.omnioffice.oms+zip` | `deck.md`: slides separated by `---`; `# title`, `- bullet`, `> speaker note`; directives `<!-- theme: ocean -->`, `<!-- layout: title -->`, `<!-- transition: fade -->`, `<!-- shape: kind x y w h [#RRGGBB] text -->` (fractions of the slide; kind may be `image` with a `media/*` entry), `<!-- slide-numbers: on -->`. Older files load unchanged |

Readers ignore unknown entries.
