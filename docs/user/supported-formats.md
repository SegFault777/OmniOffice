# Supported formats (202646b)

## OmniDoc
| Format | Read | Write | Fidelity |
|---|---|---|---|
| OMD, DOCX, ODT | yes | yes | styles, alignment, bold/italic/underline |
| RTF | yes | yes | run formatting; headings come back as bold text |
| MD | yes | yes | headings, quotes, code, inline formatting |
| TXT, HTML, XHTML, XML, LaTeX, RST, Org, Typst, MediaWiki, AsciiDoc, SGML, GML | yes | yes | edited as source text |
| DOC (Word 97-2003) | text | no | paragraphs, tables as tab-separated rows; verified with synthetic files, not yet with real Word files |
| HWP (5.x) | text | no | plain and compressed documents; password-protected and distribution documents are rejected |
| DOCM, SXW, HWPX, HML, ABW | text | no | |
| PDF, PAGES, WPD, WPS, LWP, WRI, CWK, UOF, GUL, JUNG | no | no | planned |

## OmniCel: `.omc` (native), `.csv`.  OmniShow: `.oms` (native), `.md` outline.
