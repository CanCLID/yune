# OpenCC data provenance

These text dictionaries are imported from the upstream OpenCC repository:

- Source: https://github.com/BYVoid/OpenCC/tree/master/data/dictionary
- License: Apache-2.0, as declared by upstream dictionary headers.
- Imported for Yune M15 TypeDuck `hk2s` simplifier coverage.

`HKVariants.txt` is the upstream forward Hong Kong variant dictionary; Yune derives the reverse character map needed by `hk2s` at runtime. `HKVariantsRevPhrases.txt`, `TSPhrases.txt`, and `TSCharacters.txt` are consumed directly in the `hk2s` conversion chain.
