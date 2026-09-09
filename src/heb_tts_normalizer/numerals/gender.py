"""Noun gender resolution and number-noun agreement.

Resolution order: caller overrides, then a built-in lexicon, then explicit exception
sets, then suffix heuristics. The exceptions come before the heuristics because they
exist precisely to contradict them (נשים looks masculine, מקומות looks feminine).

A leading one-letter clitic (ה/ו/ב/ל/כ/מ/ש) is stripped only as a *fallback* lookup: we
try the word as written first, so מים stays water and never becomes מ + ים.
"""

from __future__ import annotations

from ..config import Config, Gender
from . import words as w
from .convert import numeral


def noun_gender(word: str, cfg: Config) -> Gender:
    """Best guess at the grammatical gender of ``word``."""
    for form in _lookup_forms(word):
        override = cfg.gender_overrides.get(form)
        if override is not None:
            return override
    for form in _lookup_forms(word):
        known = w.LEXICON.get(form)
        if known is not None:
            return known
    for form in _lookup_forms(word):
        if form in w.FEM_DESPITE_IM or form in w.FEM_DESPITE_NO_SUFFIX:
            return Gender.FEM
        if form in w.MASC_DESPITE_OT or form in w.MASC_DESPITE_FEM_SUFFIX:
            return Gender.MASC
    return _by_suffix(word)


def count_phrase(n: int, noun: str, cfg: Config) -> str:
    """Numeral + noun, agreeing in gender and correctly ordered.

    Two Hebrew rules drive the shape: the numeral 1 *follows* its noun (ילד אחד), and a
    definite noun takes the numeral's construct form (שלושת הילדים).
    """
    gender = noun_gender(noun, cfg)
    definite = _is_definite(noun)

    if n == 1:
        return f"{noun} {numeral(1, gender)}"
    # 2 is always bound, definite or not: שני ילדים, שתי ילדות, שני הילדים.
    if n == 2 or (definite and 3 <= n <= 10):
        return f"{numeral(n, gender, construct=True)} {noun}"
    return f"{numeral(n, gender)} {noun}"


def _is_definite(noun: str) -> bool:
    return len(noun) > 2 and noun[0] == "ה"


def _lookup_forms(word: str) -> list[str]:
    """The word as written, then with one and two leading clitics peeled off."""
    forms = [word]
    current = word
    for _ in range(2):
        if len(current) > 2 and current[0] in w.CLITICS:
            current = current[1:]
            forms.append(current)
        else:
            break
    return forms


def _by_suffix(word: str) -> Gender:
    if word.endswith("ות"):
        return Gender.FEM
    if word.endswith(("ים", "יים")):
        return Gender.MASC
    if word.endswith(("ה", "ת")):
        return Gender.FEM
    return Gender.MASC
