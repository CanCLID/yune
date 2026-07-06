const DATA: &str = include_str!("m59_canonical_jyutping.tsv");

#[derive(Clone, Copy)]
pub(super) enum M59CanonicalJyutpingSection {
    Bei,
    BeingoPhrases,
    Zijiguk,
}

#[derive(Clone, Copy)]
pub(super) struct M59CanonicalJyutpingRow {
    pub(super) text: &'static str,
    pub(super) comment: &'static str,
}

pub(super) fn rows(
    section: M59CanonicalJyutpingSection,
) -> impl Iterator<Item = M59CanonicalJyutpingRow> {
    let marker = match section {
        M59CanonicalJyutpingSection::Bei => "[bei]",
        M59CanonicalJyutpingSection::BeingoPhrases => "[beingo_phrases]",
        M59CanonicalJyutpingSection::Zijiguk => "[zijiguk]",
    };
    let mut in_section = false;
    DATA.lines().filter_map(move |line| {
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line == marker;
            return None;
        }
        if !in_section {
            return None;
        }
        let (text, comment) = line
            .split_once('\t')
            .expect("M59 canonical Jyutping row should be TSV");
        Some(M59CanonicalJyutpingRow { text, comment })
    })
}
