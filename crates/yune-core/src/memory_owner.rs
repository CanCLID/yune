#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryOwnerClass {
    HeapOwnedReducible,
    HeapOwnedRequired,
    HeapOwnedGuarded,
    MmapFileBacked,
    Shared,
    SharedOrOverlapping,
    Transient,
    Unclassified,
    OverlapEstimate,
}

impl MemoryOwnerClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeapOwnedReducible => "heap_owned_reducible",
            Self::HeapOwnedRequired => "heap_owned_required",
            Self::HeapOwnedGuarded => "heap_owned_guarded",
            Self::MmapFileBacked => "mmap_file_backed",
            Self::Shared => "shared",
            Self::SharedOrOverlapping => "shared_or_overlapping",
            Self::Transient => "transient",
            Self::Unclassified => "unclassified",
            Self::OverlapEstimate => "overlap_estimate",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryOwnerRow {
    pub owner: String,
    pub class: MemoryOwnerClass,
    pub estimated_bytes: usize,
    pub item_count: usize,
    pub storage: String,
    pub notes: String,
}

impl MemoryOwnerRow {
    #[must_use]
    pub fn new(
        owner: impl Into<String>,
        class: MemoryOwnerClass,
        estimated_bytes: usize,
        item_count: usize,
        storage: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            owner: owner.into(),
            class,
            estimated_bytes,
            item_count,
            storage: storage.into(),
            notes: notes.into(),
        }
    }
}
