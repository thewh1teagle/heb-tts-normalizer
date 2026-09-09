# Soak-test corpora

Real Hebrew sentences mined from [OPUS](https://opus.nlpl.eu/) by
`scripts/mine_opus.py`. They carry no expected output — the soak test asserts
only invariants that must hold whatever the correct reading is, so no row here
was ever reviewed by a human, and none of it is a specification.

Each file is a sampled, deduplicated subset of the Hebrew side of one corpus,
under that corpus's own licence:

| File | Corpus | Licence | Source |
|---|---|---|---|
| `opensubtitles.txt` | OpenSubtitles | CC0 / public-domain dedication by the OPUS release; subtitles from opensubtitles.org | [OpenSubtitles v2024](https://object.pouta.csc.fi/OPUS-OpenSubtitles/v2024/mono/he.txt.gz) |
| `wikimedia.txt` | wikimedia (Wikipedia/Wikinews translations) | CC BY-SA 3.0 | [wikimedia v20230407](https://object.pouta.csc.fi/OPUS-wikimedia/v20230407/mono/he.txt.gz) |
| `qed.txt` | QED (educational subtitles) | CC BY-SA 3.0 | [QED v2.0a](https://object.pouta.csc.fi/OPUS-QED/v2.0a/mono/he.txt.gz) |
| `tatoeba.txt` | Tatoeba | CC BY 2.0 FR | [Tatoeba v2023-04-12](https://object.pouta.csc.fi/OPUS-Tatoeba/v2023-04-12/mono/he.txt.gz) |
| `globalvoices.txt` | GlobalVoices | CC BY 3.0 | [GlobalVoices v2018q4](https://object.pouta.csc.fi/OPUS-GlobalVoices/v2018q4/mono/he.txt.gz) |

Citations, as the corpora ask:

- OpenSubtitles — P. Lison and J. Tiedemann, OpenSubtitles2016 (LREC 2016)
- wikimedia (Wikipedia/Wikinews translations) — J. Tiedemann, Parallel Data, Tools and Interfaces in OPUS (LREC 2012)
- QED (educational subtitles) — A. Abdelali et al., The AMARA Corpus (LREC 2014)
- Tatoeba — Tatoeba Project, https://tatoeba.org
- GlobalVoices — Global Voices, https://globalvoices.org

OPUS itself: J. Tiedemann, *Parallel Data, Tools and Interfaces in OPUS*,
LREC 2012, <https://opus.nlpl.eu/>.

`manifest.json` records the exact bytes each sample came from, so
`uv run scripts/mine_opus.py --verify` can prove a sample matches its source.
