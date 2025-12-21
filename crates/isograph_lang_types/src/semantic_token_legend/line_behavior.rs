use std::ops::{Deref, Not};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum LineBehavior {
    StartsNewLine(StartsNewLineBehavior),
    EndsLine(EndsLineBehavior),
    Inline(InlineBehavior),
    IsOwnLine,
    Remove,
}

impl LineBehavior {
    pub fn starts_new_line(&self) -> bool {
        matches!(
            self,
            LineBehavior::IsOwnLine | LineBehavior::StartsNewLine(_)
        )
    }

    pub fn ends_line(&self) -> bool {
        matches!(self, LineBehavior::IsOwnLine | LineBehavior::EndsLine(_))
    }

    pub fn has_space_after(&self) -> SpaceAfter {
        match self {
            LineBehavior::StartsNewLine(starts_new_line_behavior) => {
                starts_new_line_behavior.space_after
            }
            LineBehavior::EndsLine(_) => SpaceAfter(false),
            LineBehavior::Inline(inline_behavior) => inline_behavior.space_after,
            LineBehavior::IsOwnLine => SpaceAfter(false),
            LineBehavior::Remove => SpaceAfter(false),
        }
    }

    pub fn has_space_before(&self) -> SpaceBefore {
        match self {
            LineBehavior::StartsNewLine(_) => SpaceBefore(false),
            LineBehavior::EndsLine(ends_line_behavior) => ends_line_behavior.space_before,
            LineBehavior::Inline(inline_behavior) => inline_behavior.space_before,
            LineBehavior::IsOwnLine => SpaceBefore(false),
            LineBehavior::Remove => SpaceBefore(false),
        }
    }

    pub fn should_keep(&self) -> bool {
        matches!(self, LineBehavior::Remove).not()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct StartsNewLineBehavior {
    pub space_after: SpaceAfter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct EndsLineBehavior {
    pub space_before: SpaceBefore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct InlineBehavior {
    pub space_before: SpaceBefore,
    pub space_after: SpaceAfter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SpaceBefore(pub bool);

impl Deref for SpaceBefore {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SpaceAfter(pub bool);

impl Deref for SpaceAfter {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
