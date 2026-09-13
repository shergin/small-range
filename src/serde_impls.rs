use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{OutOfRange, SmallRange, SmallRangeStorage};

#[derive(Serialize, Deserialize)]
#[serde(rename = "SmallRange")]
struct Repr {
    start: usize,
    end: usize,
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> Serialize for SmallRange<S, LEN_BITS> {
    fn serialize<Z: Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        Repr {
            start: self.start(),
            end: self.end(),
        }
        .serialize(serializer)
    }
}

impl<'de, S: SmallRangeStorage, const LEN_BITS: u32> Deserialize<'de> for SmallRange<S, LEN_BITS> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let repr = Repr::deserialize(deserializer)?;
        Self::try_new(repr.start, repr.end).ok_or_else(|| {
            D::Error::custom(OutOfRange {
                start: repr.start,
                end: repr.end,
                max_start: Self::MAX_START,
                max_len: Self::MAX_LEN,
            })
        })
    }
}
