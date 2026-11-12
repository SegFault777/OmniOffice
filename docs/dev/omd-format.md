# OMD format, version 1

An OMD file is a ZIP archive whose entries are **stored** (uncompressed) so that it can be read
and written without a compressor.

| Entry | Content |
|---|---|
| `mimetype` | `application/vnd.omnioffice.omd+zip` (first entry) |
| `manifest.json` | `{"omd":1,"generator":"OmniDoc <version>"}` |
| `content.md` | UTF-8 document text (Markdown-compatible) |

Readers must ignore unknown entries. Future versions will add `styles.json` and `media/`.
