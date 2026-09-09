"""Markdown removal — a whole-string pre-pass, not a span rule.

Markdown is punctuation for the eye. A g2p reading ``**חשוב**`` aloud has to say
something about the asterisks, so they are dropped before any rule runs. This module
therefore exports :func:`strip_markdown` (called from :mod:`..text`) instead of
``RULES``, and it is deliberately absent from the rule registry.

Judgment calls
--------------
* A fenced code block is dropped whole. Read aloud it is noise; inline ``code`` keeps
  its text because it is usually a word in the sentence.
* An image contributes nothing to speech, so ``![alt](url)`` disappears entirely,
  while ``[text](url)`` keeps ``text`` — the URL itself is unspeakable.
* Removing a list bullet or a table pipe would run two items into one breath, so a
  full stop is left behind when the item does not already end in punctuation.
* Hebrew text is full of characters Markdown also uses. ``*`` and ``_`` are stripped
  only as a *matched pair* around non-space text, and ``_`` additionally only at a
  word boundary, so ``*6555`` (a star phone number) and ``קו_תחתון`` survive intact.
  Geresh and gershayim (``׳`` ``״``) are Hebrew punctuation and are never touched.
"""

from __future__ import annotations

import regex as re

from ...text import HEB

#: Fenced code blocks, ``` or ~~~, dropped with their contents.
_FENCE = re.compile(r"^[ \t]*(?P<f>```|~~~)[^\n]*\n.*?^[ \t]*(?P=f)[^\n]*$\n?", re.M | re.S)
#: A dangling fence marker on its own line, once the pairs above are gone.
_LONE_FENCE = re.compile(r"^[ \t]*(?:```|~~~)[^\n]*$\n?", re.M)

#: <https://example.com> — an autolink says nothing out loud.
_AUTOLINK = re.compile(r"<(?:https?|mailto|tel):[^>\s]*>")

_IMAGE = re.compile(r"!\[[^\]\n]*\]\([^)\n]*\)")
_INLINE_LINK = re.compile(r"\[([^\]\n]*)\]\([^)\n]*\)")
_REF_LINK = re.compile(r"\[([^\]\n]*)\]\[[^\]\n]*\]")
#: A link definition line, ``[ref]: https://…``, is metadata and never spoken.
_LINK_DEF = re.compile(r"^[ \t]*\[[^\]\n]+\]:[^\n]*$\n?", re.M)

_INLINE_CODE = re.compile(r"(?<!`)(`+)(?!`)([^`\n]+?)\1(?!`)")

_HEADING = re.compile(r"^[ \t]{0,3}#{1,6}[ \t]+(?P<text>[^\n]*?)(?:[ \t]+#+)?[ \t]*$")
_THEMATIC_BREAK = re.compile(r"^[ \t]{0,3}(?P<c>[-*_])(?:[ \t]*(?P=c)){2,}[ \t]*$")
_BLOCKQUOTE = re.compile(r"^[ \t]{0,3}>+[ \t]?")
_LIST_MARKER = re.compile(r"^[ \t]*(?:[-*+]|\d{1,9}[.)])[ \t]+")
_TABLE_DIVIDER = re.compile(r"^[ \t]*\|?[ \t]*:?-{2,}:?[ \t]*(?:\|[ \t]*:?-+:?[ \t]*)*\|?[ \t]*$")

_STRIKE = re.compile(r"~~(?=\S)(.+?)(?<=\S)~~", re.S)
_BOLD_STAR = re.compile(r"\*\*(?=\S)(.+?)(?<=\S)\*\*", re.S)
_BOLD_UNDER = re.compile(rf"(?<![\w{HEB}])__(?=\S)(.+?)(?<=\S)__(?![\w{HEB}])", re.S)
_ITALIC_STAR = re.compile(r"\*(?=[^\s*])([^*\n]+?)(?<=\S)\*")
_ITALIC_UNDER = re.compile(rf"(?<![\w{HEB}])_(?=\S)([^_\n]+?)(?<=\S)_(?![\w{HEB}])")

_ESCAPED = re.compile(r"\\([\\`*_{}\[\]()#+\-.!~>|])")
#: Escaped punctuation is parked in the private-use area so the passes below cannot
#: mistake ``\*`` for emphasis; ``_unprotect`` puts the character back at the end.
_PUA = 0xE000
_PROTECTED = re.compile("[\\ue000-\\ue07f]")


def _protect_escapes(text: str) -> str:
    return _ESCAPED.sub(lambda m: chr(_PUA + ord(m.group(1))), text)


#: Escaped emphasis markers are dropped rather than restored. A literal "*" says nothing
#: aloud, and restoring one is not idempotent: the next pass reads it as emphasis again.
_DROPPED_ESCAPES = {_PUA + ord(ch) for ch in "*_"}


def _unprotect_escapes(text: str) -> str:
    def restore(m: re.Match[str]) -> str:
        ch = m.group(0)
        return "" if ord(ch) in _DROPPED_ESCAPES else chr(ord(ch) - _PUA)

    return _PROTECTED.sub(restore, text)


#: Punctuation that already ends a phrase, so no full stop needs adding.
_ENDS_PHRASE = re.compile(r"[.,;:!?…׃־]$")


def _terminate(text: str) -> str:
    """Give a bullet or table row a sentence boundary so the TTS breathes between items."""
    text = text.strip()
    if not text or _ENDS_PHRASE.search(text):
        return text
    return text + "."


def _table_row(line: str) -> str | None:
    """Cell text of a Markdown table row, or None if the line is not one."""
    stripped = line.strip()
    if "|" not in stripped:
        return None
    if not (stripped.startswith("|") or stripped.endswith("|")):
        return None
    if _TABLE_DIVIDER.match(stripped):
        return ""
    cells = [c.strip() for c in stripped.strip("|").split("|")]
    return _terminate(", ".join(c for c in cells if c))


def _strip_block_syntax(text: str) -> str:
    """Line-level Markdown: headings, rules, quotes, bullets, tables."""
    out: list[str] = []
    for raw in text.split("\n"):
        line = raw
        if _THEMATIC_BREAK.match(line):
            out.append("")
            continue
        heading = _HEADING.match(line)
        if heading is not None:
            out.append(heading.group("text").strip())
            continue
        row = _table_row(line)
        if row is not None:
            out.append(row)
            continue
        line = _BLOCKQUOTE.sub("", line)
        item = _LIST_MARKER.match(line)
        if item is not None:
            out.append(_terminate(line[item.end() :]))
            continue
        out.append(line)
    return "\n".join(out)


def strip_emphasis(text: str) -> str:
    """Drop bold/italic/strikethrough markers, keeping the words they wrap."""
    text = _STRIKE.sub(r"\1", text)
    # Bold first: ``***x***`` is then left as ``*x*`` for the italic pass.
    text = _BOLD_STAR.sub(r"\1", text)
    text = _BOLD_UNDER.sub(r"\1", text)
    text = _ITALIC_STAR.sub(r"\1", text)
    text = _ITALIC_UNDER.sub(r"\1", text)
    return text


def strip_links(text: str) -> str:
    """``[text](url)`` -> ``text``; images, autolinks and link definitions -> nothing."""
    text = _LINK_DEF.sub("", text)
    text = _IMAGE.sub("", text)
    text = _INLINE_LINK.sub(r"\1", text)
    text = _REF_LINK.sub(r"\1", text)
    text = _AUTOLINK.sub("", text)
    return text


def strip_code(text: str) -> str:
    """Drop fenced blocks entirely; keep the text inside inline code spans."""
    text = _FENCE.sub("", text)
    text = _LONE_FENCE.sub("", text)
    return _INLINE_CODE.sub(r"\2", text)


def strip_markdown(text: str) -> str:
    """Remove Markdown syntax, leaving only the words and the prosodic punctuation."""
    text = _protect_escapes(text)
    text = strip_code(text)
    text = strip_links(text)
    text = _strip_block_syntax(text)
    text = strip_emphasis(text)
    # An escaped ``\*`` was never markup, so it comes back as itself, unspoken as syntax.
    return _unprotect_escapes(text)


__all__ = ["strip_code", "strip_emphasis", "strip_links", "strip_markdown"]
