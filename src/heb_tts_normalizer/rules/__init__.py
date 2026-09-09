"""Rule registry. A rule folder is self-contained; adding one means adding it here."""

from ..scanner import Rule
from .abbrev.rule import RULES as ABBREV
from .currency.rule import RULES as CURRENCY
from .dates.rule import RULES as DATES
from .numbers.rule import RULES as NUMBERS
from .percent.rule import RULES as PERCENT
from .phones.rule import RULES as PHONES
from .protect.rule import RULES as PROTECT
from .ranges.rule import RULES as RANGES
from .times.rule import RULES as TIMES
from .units.rule import RULES as UNITS

#: Every span rule, in no particular order — the scanner sorts by priority.
#: ``cleanup`` is deliberately absent: it rewrites the whole string, so it runs
#: as a pre-pass in ``text.clean`` rather than as a span rule.
ALL_RULES: list[Rule] = [
    *PROTECT,
    *DATES,
    *TIMES,
    *PHONES,
    *CURRENCY,
    *PERCENT,
    *RANGES,
    *UNITS,
    *ABBREV,
    *NUMBERS,
]

__all__ = ["ALL_RULES"]
