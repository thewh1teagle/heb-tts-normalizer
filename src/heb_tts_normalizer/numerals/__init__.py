"""The Hebrew numeral engine: numbers, agreement and gender, for every rule to share."""

from .convert import decimal, digits, numeral, ordinal
from .gender import count_phrase, noun_gender

__all__ = ["count_phrase", "decimal", "digits", "noun_gender", "numeral", "ordinal"]
