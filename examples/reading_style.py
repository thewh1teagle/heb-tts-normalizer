"""The same text read two ways.

Reading style is a property of the caller, not of the text: a news bulletin wants a
24-hour clock, a voice assistant wants "שתיים וחצי".

    uv run examples/reading_style.py
"""

from heb_tts_normalizer import Clock, Config, DateOrder, Gender, normalize

TEXT = "הטיסה ב-14:30 בתאריך 03/04/2026, והמשקל 2 ק״ג"

print("default (12-hour, day/month):")
print(" ", normalize(TEXT))

print("\n24-hour clock, month/day:")
print(" ", normalize(TEXT, Config(clock=Clock.H24, date_order=DateOrder.MDY)))

print("\nunits left alone, for a pipeline that expands them itself:")
print(" ", normalize(TEXT, Config(expand_units=False)))

# Gender agreement comes from a lexicon plus morphology, and neither can know a
# loanword or a term of art. Declaring one makes it countable and fixes its agreement.
print("\nteaching it a word it cannot know:")
print(" ", normalize("3 סטוריז"))
print(" ", normalize("3 סטוריז", Config(gender_overrides={"סטוריז": Gender.MASC})))
